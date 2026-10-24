use crate::{
    channel,
    envelope::{HANDSHAKE, STATELESS, initialize, request},
    rpc::{Endpoint, response, response_as},
};
use reqwest::StatusCode;
use rmcp::serde_json::{Value, json};
use web_rs::{
    VERSION,
    cli::WorkMode,
    config,
    mcp::tools::{ToolCredentials, ToolService},
};
fn assert_strict_output(root: &Value) {
    let mut pending = vec![root];
    while let Some(value) = pending.pop() {
        if let Some(object) = value.as_object() {
            for forbidden in ["$defs", "$ref"] {
                assert!(!object.contains_key(forbidden), "{forbidden}: {root}");
            }
            if object.contains_key("type") || object.contains_key("anyOf") {
                for metadata in ["title", "description"] {
                    assert!(!object.contains_key(metadata), "{metadata}: {root}");
                }
            }
            if object.get("type").and_then(Value::as_str) == Some("object") {
                assert_eq!(
                    object.get("additionalProperties"),
                    Some(&json!(false)),
                    "{root}"
                );
            }
            pending.extend(object.values());
        } else if let Some(array) = value.as_array() {
            pending.extend(array);
        }
    }
}
#[tokio::test]
async fn discovery_and_tool_schemas_follow_configuration_in_json_and_sse_without_version_leaks() {
    for (json_response, chunking) in [(true, true), (false, true), (true, false), (false, false)] {
        let mut settings = config::load_embedded().unwrap();
        settings.server.json_response = json_response;
        settings.chunking.enabled = chunking;
        settings.server.name = "configured-web".to_owned();
        settings.tools.search_query = "lookup".to_owned();
        settings.tools.open = "browse".to_owned();
        settings.tools.find = "scan".to_owned();
        settings.protocol.tools_list_cache.ttl_ms = 72_000;
        let endpoint = Endpoint::configured(settings).await;
        let discover = request(STATELESS, "server/discover", json!({}));
        let info = response_as(
            endpoint.post(&discover, STATELESS),
            StatusCode::OK,
            json_response,
        )
        .await;
        assert_eq!(
            info.pointer("/result/supportedVersions"),
            Some(&json!([HANDSHAKE, STATELESS]))
        );
        assert_eq!(
            info.pointer("/result/_meta/io.modelcontextprotocol~1serverInfo/name"),
            Some(&json!("configured-web"))
        );
        assert_eq!(
            info.pointer("/result/_meta/io.modelcontextprotocol~1serverInfo/version"),
            Some(&json!(VERSION))
        );
        let modern = request(STATELESS, "tools/list", json!({}));
        let sessionless = response_as(
            endpoint.post(&modern, STATELESS),
            StatusCode::OK,
            json_response,
        )
        .await;
        let initialized = response(
            endpoint
                .client
                .post(format!("{}/mcp", endpoint.server.address))
                .header("accept", "application/json, text/event-stream")
                .json(&initialize(HANDSHAKE)),
            StatusCode::OK,
        )
        .await;
        assert_eq!(
            initialized.pointer("/result/protocolVersion"),
            Some(&json!(HANDSHAKE))
        );
        let old = request(HANDSHAKE, "tools/list", json!({}));
        let (handshake, stateless) = tokio::join!(
            response_as(
                endpoint.post(&old, HANDSHAKE),
                StatusCode::OK,
                json_response
            ),
            response_as(
                endpoint.post(&modern, STATELESS),
                StatusCode::OK,
                json_response
            ),
        );
        assert_eq!(sessionless, stateless);
        assert_eq!(
            handshake.pointer("/result/tools"),
            stateless.pointer("/result/tools")
        );
        assert!(handshake.pointer("/result/ttlMs").is_none());
        assert_eq!(stateless.pointer("/result/ttlMs"), Some(&json!(72_000_u64)));
        assert_eq!(
            stateless.pointer("/result/cacheScope"),
            Some(&json!("public"))
        );
        let tools = stateless
            .pointer("/result/tools")
            .unwrap()
            .as_array()
            .unwrap();
        assert_eq!(
            tools
                .iter()
                .map(|tool| tool.get("name").unwrap().as_str().unwrap())
                .collect::<Vec<_>>(),
            ["lookup", "browse", "scan"]
        );
        for (tool, path) in tools.iter().zip([
            "/properties/results/items/properties/highlight/type",
            "/properties/results/items/anyOf/0/properties/page/properties/content/type",
            "/properties/pages/items/properties/matches/items/properties/snippet/type",
        ]) {
            assert!(tool.pointer("/inputSchema/properties/requests").is_some());
            let schema = tool.get("outputSchema").unwrap();
            assert_eq!(schema.pointer(path), Some(&json!("string")));
            assert_strict_output(schema);
            if tool.get("name") != Some(&json!("lookup")) {
                let encoded = sonic_rs::to_string(schema).unwrap();
                for field in ["\"chunk\"", "\"total_chunks\""] {
                    assert_eq!(encoded.contains(field), chunking, "{schema}");
                }
            }
            if tool.get("name") == Some(&json!("browse")) {
                let input = sonic_rs::to_string(tool.get("inputSchema").unwrap()).unwrap();
                assert_eq!(input.contains("\"chunk\""), chunking, "{input}");
            }
        }
    }
}
#[tokio::test]
async fn filesystem_discovery_exposes_file_contracts_without_response_only_tools() {
    let mut settings = config::load_embedded().unwrap();
    settings.tools.search_query = "lookup".to_owned();
    settings.tools.open = "browse".to_owned();
    let service = ToolService::new_with_credentials(
        settings,
        ToolCredentials::default(),
        WorkMode::Filesystem,
    )
    .unwrap();
    let listed = channel::call(service, "tools/list", json!({})).await;
    let tools = listed.pointer("/result/tools").unwrap().as_array().unwrap();
    assert_eq!(
        tools
            .iter()
            .map(|tool| tool.get("name").unwrap().as_str().unwrap())
            .collect::<Vec<_>>(),
        ["lookup", "browse"]
    );
    for tool in tools {
        assert!(
            tool.pointer("/inputSchema/properties/output_path")
                .is_some()
        );
        let schema = tool.get("outputSchema").unwrap();
        let properties = schema.get("properties").unwrap().as_object().unwrap();
        assert_eq!(properties.len(), 3);
        for key in ["output_path", "error", "warning"] {
            assert_eq!(
                properties.get(key).unwrap().get("type"),
                Some(&json!("string"))
            );
        }
        assert_strict_output(schema);
    }
}
