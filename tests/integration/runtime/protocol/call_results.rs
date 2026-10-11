use crate::{
    channel,
    envelope::{HANDSHAKE, STATELESS, request},
    listener::LocalServer,
    rpc::{Endpoint, response},
    upstream::Upstream,
};
use axum::{Router, routing::post};
use core::time::Duration;
use reqwest::StatusCode;
use rmcp::serde_json::json;
use web_rs::{
    cli::WorkMode,
    config,
    mcp::tools::{ToolCredentials, ToolService},
    page::reader::ReaderCredentials,
    search::SearchCredentials,
};
#[tokio::test]
async fn application_and_protocol_failures_keep_their_shapes_and_hide_internal_details() {
    let endpoint = Endpoint::configured(config::load_embedded().unwrap()).await;
    for version in [HANDSHAKE, STATELESS] {
        let unknown = request(
            version,
            "tools/call",
            json ! ({ "name" : "absent" , "arguments" : { } }),
        );
        let protocol = response(endpoint.post(&unknown, version), StatusCode::OK).await;
        assert_eq!(protocol.pointer("/error/code"), Some(&json!(-32602_i64)));
        assert!(protocol.get("result").is_none());
    }
    for (name, arguments, expected) in [
        (
            "search_query",
            json ! ({ "category" : "news" }),
            "q is required",
        ),
        (
            "search_query",
            json ! ({ "requests" : [{ "q" : "one" }] }),
            "API key",
        ),
        ("open", json ! ({ "chunk" : 1_u64 }), "url is required"),
        (
            "find",
            json ! ({ "requests" : [{ "url" : "https://example.org" , "pattern" : "[" }] }),
            "regular expression",
        ),
    ] {
        let body = request(
            STATELESS,
            "tools/call",
            json ! ({ "name" : name , "arguments" : arguments }),
        );
        let reply = response(endpoint.post(&body, STATELESS), StatusCode::OK).await;
        assert_eq!(
            reply.pointer("/result/isError"),
            Some(&json!(true)),
            "{reply}"
        );
        assert!(
            reply
                .pointer("/result/content/0/text")
                .unwrap()
                .as_str()
                .unwrap()
                .contains(expected),
            "{reply}"
        );
        assert!(reply.get("error").is_none());
        assert!(reply.pointer("/result/structuredContent").is_none());
    }
    let upstream = Upstream::json(&json!({})).await;
    let service = ToolService::new_with_credentials(
        upstream.server.settings(),
        ToolCredentials {
            search: None,
            reader: Some(ReaderCredentials::Jina("private-key\ninvalid".to_owned())),
        },
        WorkMode::Response,
    )
    .unwrap();
    let internal = channel :: call (service , "tools/call" , json ! ({ "name" : "find" , "arguments" : { "requests" : [{ "url" : format ! ("{}/unavailable" , upstream . server . address) , "pattern" : "needle" }] } }) ,) . await ;
    assert_eq!(
        internal.pointer("/error/code"),
        Some(&json!(-32603_i64)),
        "{internal}"
    );
    assert_eq!(
        internal.pointer("/error/message"),
        Some(&json!(
            "Unexpected server error. Retry the request or contact the service operator."
        ))
    );
    assert!(internal.get("result").is_none());
    assert!(
        !sonic_rs::to_string(&internal)
            .unwrap()
            .contains("private-key")
    );
    assert!(upstream.requests.lock().await.is_empty());
    let slow = LocalServer::start(Router::new().route(
        "/api",
        post(|| async {
            tokio::time::sleep(Duration::from_secs(2)).await;
            "{\"results\":[]}"
        }),
    ))
    .await;
    let mut settings = slow.settings();
    settings.http.timeout_seconds = 0.1_f64;
    let delayed = ToolService::new_with_credentials(
        settings,
        ToolCredentials {
            search: Some(SearchCredentials::Exa("key".to_owned())),
            reader: None,
        },
        WorkMode::Response,
    )
    .unwrap();
    let recoverable = channel::call(
        delayed,
        "tools/call",
        json ! ({ "name" : "search_query" , "arguments" : { "requests" : [{ "q" : "test" }] } }),
    )
    .await;
    assert_eq!(
        recoverable.pointer("/result/isError"),
        Some(&json!(true)),
        "{recoverable}"
    );
    assert!(
        recoverable
            .pointer("/result/content/0/text")
            .unwrap()
            .as_str()
            .unwrap()
            .contains("timed out")
    );
    assert!(recoverable.get("error").is_none());
    assert!(recoverable.pointer("/result/structuredContent").is_none());
}
