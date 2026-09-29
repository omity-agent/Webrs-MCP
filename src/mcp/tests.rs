use crate::{
    VERSION,
    cli::WorkMode,
    config,
    mcp::{
        schemas, stdio_service, streamable_http_config, tools::ToolCredentials, tools::ToolService,
    },
};
use rmcp::{ServerHandler, model::ProtocolVersion};
#[test]
fn rmcp_tools_expose_expected_schemas() {
    let tools = schemas::tools(WorkMode::Response).unwrap_or_else(|error| panic!("{error}"));
    let names = tools
        .iter()
        .map(|tool| tool.name.as_ref())
        .collect::<Vec<_>>();
    assert_eq!(names, ["search_query", "open", "find"]);
    let output_shapes = [
        (
            "results",
            "/properties/results/items/properties/highlight/type",
        ),
        (
            "results",
            "/properties/results/items/anyOf/0/properties/page/properties/content/type",
        ),
        (
            "pages",
            "/properties/pages/items/properties/matches/items/properties/snippet/type",
        ),
    ];
    for (tool, (expected_property, nested_type_path)) in tools.into_iter().zip(output_shapes) {
        assert!(tool.input_schema.contains_key("properties"));
        let output_schema = tool
            .output_schema
            .unwrap_or_else(|| panic!("{} is missing outputSchema", tool.name));
        assert_eq!(
            output_schema
                .get("type")
                .and_then(rmcp::serde_json::Value::as_str),
            Some("object")
        );
        let properties = output_schema
            .get("properties")
            .and_then(rmcp::serde_json::Value::as_object)
            .unwrap_or_else(|| panic!("{} outputSchema has no properties", tool.name));
        assert!(properties.contains_key(expected_property));
        assert_eq!(
            rmcp::serde_json::Value::Object(output_schema.as_ref().clone())
                .pointer(nested_type_path)
                .and_then(rmcp::serde_json::Value::as_str),
            Some("string")
        );
        assert_inline_strict_schema(&rmcp::serde_json::Value::Object(
            output_schema.as_ref().clone(),
        ));
    }
}
#[test]
fn filesystem_mode_exposes_only_file_backed_tools() {
    let tools = schemas::tools(WorkMode::Filesystem).unwrap_or_else(|error| panic!("{error}"));
    let names = tools
        .iter()
        .map(|tool| tool.name.as_ref())
        .collect::<Vec<_>>();
    assert_eq!(names, ["search_query", "open"]);
    for tool in tools {
        let properties = tool
            .input_schema
            .get("properties")
            .and_then(rmcp::serde_json::Value::as_object)
            .unwrap_or_else(|| panic!("{} inputSchema has no properties", tool.name));
        assert!(properties.contains_key("output_path"));
        let output = tool
            .output_schema
            .unwrap_or_else(|| panic!("{} is missing outputSchema", tool.name));
        let response_properties = output
            .get("properties")
            .and_then(rmcp::serde_json::Value::as_object)
            .unwrap_or_else(|| panic!("{} outputSchema has no properties", tool.name));
        assert_eq!(response_properties.len(), 3);
        assert!(response_properties.contains_key("output_path"));
        assert!(response_properties.contains_key("error"));
        assert!(response_properties.contains_key("warning"));
    }
}
#[expect(
    clippy::pattern_type_mismatch,
    reason = "Matching borrowed JSON variants avoids cloning the schema tree."
)]
fn assert_inline_strict_schema(value: &rmcp::serde_json::Value) {
    match value {
        rmcp::serde_json::Value::Object(object) => {
            assert!(!object.contains_key("$defs"));
            assert!(!object.contains_key("$ref"));
            assert_ne!(
                object
                    .get("format")
                    .and_then(rmcp::serde_json::Value::as_str),
                Some("uint")
            );
            assert_ne!(
                object
                    .get("format")
                    .and_then(rmcp::serde_json::Value::as_str),
                Some("uint64")
            );
            if object.get("type").and_then(rmcp::serde_json::Value::as_str) == Some("object") {
                assert_eq!(
                    object.get("additionalProperties"),
                    Some(&rmcp::serde_json::Value::Bool(false))
                );
            }
            for nested in object.values() {
                assert_inline_strict_schema(nested);
            }
        }
        rmcp::serde_json::Value::Array(values) => {
            for nested in values {
                assert_inline_strict_schema(nested);
            }
        }
        rmcp::serde_json::Value::Null
        | rmcp::serde_json::Value::Bool(_)
        | rmcp::serde_json::Value::Number(_)
        | rmcp::serde_json::Value::String(_) => {}
    }
}
#[test]
fn rmcp_server_info_uses_embedded_identity() {
    let config = config::load_embedded().unwrap_or_else(|error| panic!("{error}"));
    let service = ToolService::new(config).unwrap_or_else(|error| panic!("{error}"));
    let info = ServerHandler::get_info(&service);
    assert_eq!(info.protocol_version, ProtocolVersion::V_2026_07_28);
    assert_eq!(
        ServerHandler::supported_protocol_versions(&service).as_ref(),
        &[ProtocolVersion::V_2026_07_28]
    );
    assert_eq!(info.server_info.name, "web");
    assert_eq!(info.server_info.version, VERSION);
    assert!(info.capabilities.tools.is_some());
}
#[test]
fn streamable_http_uses_the_modern_stateless_lifecycle() {
    let config = config::load_embedded().unwrap_or_else(|error| panic!("{error}"));
    let http = streamable_http_config(&config);
    assert!(!http.legacy_session_mode);
    assert!(http.stateless_protocol_metadata_required);
}
#[test]
fn stdio_service_allows_private_network_urls() {
    let config = config::load_embedded().unwrap_or_else(|error| panic!("{error}"));
    let service = stdio_service(
        &config,
        WorkMode::Response,
        ToolCredentials {
            search: Some(crate::search::SearchCredentials::Exa("exa-key".to_owned())),
            reader: None,
        },
    )
    .unwrap_or_else(|error| panic!("{error}"));
    assert!(!service.config().ssrf.block_private_networks);
    assert!(!service.config().ssrf.block_local_hostnames);
    assert!(config.ssrf.block_private_networks);
    assert!(config.ssrf.block_local_hostnames);
}
