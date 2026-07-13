use crate::{
    VERSION, config,
    mcp::{schemas, stdio_service, tools::ToolCredentials, tools::ToolService},
};
use rmcp::ServerHandler;
#[test]
fn rmcp_tools_expose_expected_schemas() {
    let tools = schemas::tools().unwrap_or_else(|error| panic!("{error}"));
    let names = tools
        .iter()
        .map(|tool| tool.name.as_ref())
        .collect::<Vec<_>>();
    assert_eq!(names, ["search_query", "open", "find"]);
    let output_properties = ["results", "results", "pages"];
    for (tool, expected_property) in tools.into_iter().zip(output_properties) {
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
    }
}
#[test]
fn rmcp_server_info_uses_embedded_identity() {
    let config = config::load_embedded().unwrap_or_else(|error| panic!("{error}"));
    let service = ToolService::new(config).unwrap_or_else(|error| panic!("{error}"));
    let info = ServerHandler::get_info(&service);
    assert_eq!(info.protocol_version.as_str(), "2025-06-18");
    assert_eq!(info.server_info.name, "web");
    assert_eq!(info.server_info.version, VERSION);
    assert!(info.capabilities.tools.is_some());
}
#[test]
fn stdio_service_allows_private_network_urls() {
    let config = config::load_embedded().unwrap_or_else(|error| panic!("{error}"));
    let service = stdio_service(
        &config,
        ToolCredentials {
            exa_api_key: Some("exa-key".to_owned()),
            reader: None,
        },
    )
    .unwrap_or_else(|error| panic!("{error}"));
    assert!(!service.config().ssrf.block_private_networks);
    assert!(!service.config().ssrf.block_local_hostnames);
    assert!(config.ssrf.block_private_networks);
    assert!(config.ssrf.block_local_hostnames);
}
