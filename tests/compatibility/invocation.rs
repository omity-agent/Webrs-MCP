use super::wire::{Endpoint, LEGACY, MODERN, request, response};
use alloc::sync::Arc;
use axum::{Json, Router, extract::State, http::HeaderMap, routing::post};
use reqwest::StatusCode;
use rmcp::serde_json::{Value, json};
use tokio::sync::Mutex;
use web_rs::config;
async fn search(State(keys): State<Arc<Mutex<Vec<String>>>>, headers: HeaderMap) -> Json<Value> {
    keys.lock().await.push(
        headers
            .get("x-api-key")
            .unwrap()
            .to_str()
            .unwrap()
            .to_owned(),
    );
    Json(
        json ! ({ "results" : [{ "url" : "https://example.org/" , "title" : "Test" , "highlights" : ["Found"] }] }),
    )
}
#[tokio::test]
async fn tool_calls_keep_credentials_and_result_shapes_isolated() {
    let keys = Arc::new(Mutex::new(Vec::new()));
    let upstream = Endpoint::listen(
        Router::new()
            .route("/mcp", post(search))
            .with_state(Arc::clone(&keys)),
    )
    .await;
    let mut config = config::load_embedded().unwrap();
    config.ssrf.block_private_networks = false;
    config.ssrf.block_local_hostnames = false;
    config.search.exa.endpoint.clone_from(&upstream.url);
    let endpoint = Endpoint::configured(config).await;
    let mut results = Vec::new();
    for version in [LEGACY, MODERN] {
        let body = request(
            version,
            "tools/call",
            json ! ({ "name" : "search_query" , "arguments" : { "requests" : [{ "q" : "test" }] } }),
        );
        let reply = response(
            endpoint
                .post(&body, Some(version))
                .header("x-exa-api-key", version),
            StatusCode::OK,
        )
        .await;
        assert_eq!(reply.pointer("/result/isError"), Some(&json!(false)));
        assert!(reply.pointer("/result/content/0/text").is_some());
        assert_eq!(
            reply.pointer("/result/resultType"),
            (version == MODERN).then_some(&json!("complete"))
        );
        results.push(reply.pointer("/result/structuredContent").unwrap().clone());
        let missing_key = response(endpoint.post(&body, Some(version)), StatusCode::OK).await;
        assert_eq!(missing_key.pointer("/result/isError"), Some(&json!(true)));
    }
    assert_eq!(results.first(), results.get(1));
    assert_eq!(*keys.lock().await, [LEGACY, MODERN]);
}
#[tokio::test]
async fn unknown_tools_are_protocol_errors_in_both_versions() {
    let endpoint = Endpoint::new(true).await;
    for version in [LEGACY, MODERN] {
        let body = request(
            version,
            "tools/call",
            json ! ({ "name" : "absent" , "arguments" : { } }),
        );
        let status = if version == MODERN {
            StatusCode::BAD_REQUEST
        } else {
            StatusCode::OK
        };
        let reply = response(endpoint.post(&body, Some(version)), status).await;
        assert_eq!(reply.pointer("/error/code"), Some(&json!(-32602_i64)));
    }
}
