use super::harness::{PAGE, Site, renamed_config};
use axum::{
    Json, Router,
    routing::{get, post},
};
use rmcp::serde_json::{Value, json};
use web_rs::{app, cli::WorkMode, mcp::schemas};
pub(super) fn request(method: &str, mut params: Value) -> Value {
    params . as_object_mut () . unwrap () . insert ("_meta" . to_owned () , json ! ({ "io.modelcontextprotocol/protocolVersion" : "2026-07-28" , "io.modelcontextprotocol/clientCapabilities" : { } })) ;
    json ! ({ "jsonrpc" : "2.0" , "id" : 1_u64 , "method" : method , "params" : params })
}
async fn http_call(address: &str, method: &str, params: Value) -> Value {
    let body = request(method, params);
    let mut builder = reqwest::Client::builder()
        .no_proxy()
        .build()
        .unwrap()
        .post(format!("{address}/mcp"))
        .header("accept", "application/json, text/event-stream")
        .header("mcp-protocol-version", "2026-07-28")
        .header("mcp-method", method)
        .header("x-exa-api-key", "test-key");
    if let Some(name) = body.pointer("/params/name").and_then(Value::as_str) {
        builder = builder.header("mcp-name", name);
    }
    let response = builder.json(&body).send().await.unwrap();
    assert!(response.status().is_success());
    let text = response.text().await.unwrap();
    sonic_rs::from_str(&text).unwrap()
}
#[test]
fn schemas_take_all_exposed_names_from_configuration() {
    let settings = renamed_config();
    for (mode, expected) in [
        (WorkMode::Response, vec!["lookup", "browse", "scan"]),
        (WorkMode::Filesystem, vec!["lookup", "browse"]),
    ] {
        let tools = schemas::tools(&settings.tools, mode).unwrap();
        let names = tools
            .iter()
            .map(|tool| tool.name.as_ref())
            .collect::<Vec<_>>();
        assert_eq!(names, expected);
        for name in expected {
            assert!(
                schemas::tool_by_name(&settings.tools, mode, name)
                    .unwrap()
                    .is_some()
            );
        }
    }
}
#[tokio::test]
async fn http_registers_and_dispatches_all_three_configured_tools() {
    let upstream = Site :: listen (Router :: new () . route ("/query" , post (| | async { Json (json ! ({ "results" : [{ "url" : "https://example.org" , "highlights" : ["Found"] }] })) })) . route ("/page" , get (| | async { ([("content-type" , "text/markdown")] , PAGE) })) ,) . await ;
    let mut settings = renamed_config();
    settings.ssrf.block_private_networks = false;
    settings.ssrf.block_local_hostnames = false;
    settings.search.exa.endpoint = format!("{}/query", upstream.address);
    let service = Site::listen(app::router(settings).unwrap()).await;
    let listed = http_call(&service.address, "tools/list", json!({})).await;
    let names = listed
        .pointer("/result/tools")
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .map(|tool| tool.get("name").unwrap().as_str().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(names, ["lookup", "browse", "scan"]);
    let url = format!("{}/page", upstream.address);
    for (name, arguments, property) in [
        (
            "lookup",
            json ! ({ "requests" : [{ "q" : "one" }] }),
            "results",
        ),
        (
            "browse",
            json ! ({ "requests" : [{ "url" : url , "chunk" : 0_u64 }] }),
            "results",
        ),
        (
            "scan",
            json ! ({ "requests" : [{ "url" : url , "pattern" : "needle" }] }),
            "pages",
        ),
    ] {
        let reply = http_call(
            &service.address,
            "tools/call",
            json ! ({ "name" : name , "arguments" : arguments }),
        )
        .await;
        assert_eq!(reply.pointer("/result/isError"), Some(&json!(false)));
        assert!(
            reply
                .pointer("/result/structuredContent")
                .unwrap()
                .get(property)
                .is_some()
        );
    }
}
