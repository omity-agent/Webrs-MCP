use super::fixture::{Listener, Upstream};
use clap::Parser as _;
use http::{HeaderMap, HeaderValue};
use rmcp::serde_json::{Value, json};
use web_rs::{app, cli::Cli, search::SearchCredentials};
#[test]
fn cli_selects_exactly_one_backend() {
    for (flag, expected) in [
        ("--exa-api-key", SearchCredentials::Exa("key".to_owned())),
        (
            "--octen-api-key",
            SearchCredentials::Octen("key".to_owned()),
        ),
    ] {
        let options = Cli::try_parse_from(["web-rs", "--transport", "stdio", flag, "key"])
            .unwrap()
            .runtime_options()
            .unwrap();
        assert_eq!(options.credentials.search, Some(expected));
    }
    for arguments in [
        vec!["web-rs", "--exa-api-key", "exa", "--octen-api-key", "octen"],
        vec!["web-rs", "--octen-api-key", ""],
        vec!["web-rs", "--exa-api-key", "   "],
    ] {
        let error = Cli::try_parse_from(arguments)
            .unwrap()
            .runtime_options()
            .unwrap_err();
        assert!(error.client_message().contains("API key"));
    }
}
async fn call(server: &Listener, headers: HeaderMap) -> Value {
    let response = reqwest :: Client :: builder () . no_proxy () . build () . unwrap () . post (format ! ("{}/mcp" , server . address)) . headers (headers) . header ("MCP-Protocol-Version" , "2026-07-28") . header ("Accept" , "application/json, text/event-stream") . json (& json ! ({ "jsonrpc" : "2.0" , "id" : 1_u64 , "method" : "tools/call" , "params" : { "_meta" : { "io.modelcontextprotocol/protocolVersion" : "2026-07-28" } , "name" : "search_query" , "arguments" : { "requests" : [{ "q" : "test" , "category" : "news" }] } } })) . send () . await . unwrap () ;
    let status = response.status();
    let body = response.text().await.unwrap();
    assert!(status.is_success(), "{status}: {body}");
    sonic_rs::from_str(&body).unwrap()
}
#[tokio::test]
async fn http_selects_backend_and_propagates_warnings() {
    let upstream = Upstream :: json (& json ! ({ "code" : 0_i64 , "msg" : "ok" , "data" : { "results" : [] } , "meta" : { "warning" : "Upstream warning" } })) . await ;
    let server = Listener::start(app::router(upstream.config()).unwrap()).await;
    let mut headers = HeaderMap::new();
    headers.insert("x-octen-api-key", HeaderValue::from_static("octen"));
    let response = call(&server, headers).await;
    assert_ne!(response.pointer("/result/isError"), Some(&json!(true)));
    let warnings = response
        .pointer("/result/structuredContent/warning")
        .unwrap()
        .as_array()
        .unwrap();
    assert!(
        warnings
            .iter()
            .any(|warning| warning.as_str().unwrap().contains("category"))
    );
    assert!(
        warnings
            .iter()
            .any(|warning| warning.as_str().unwrap().contains("Upstream warning"))
    );
    assert_eq!(upstream.requests.lock().await.len(), 1);
}
#[tokio::test]
async fn http_rejects_missing_duplicate_empty_or_conflicting_keys() {
    let upstream = Upstream::json(&json!({})).await;
    let server = Listener::start(app::router(upstream.config()).unwrap()).await;
    let mut conflicting = HeaderMap::new();
    conflicting.insert("x-exa-api-key", HeaderValue::from_static("exa"));
    conflicting.insert("x-octen-api-key", HeaderValue::from_static("octen"));
    let mut duplicate = HeaderMap::new();
    duplicate.append("x-octen-api-key", HeaderValue::from_static("one"));
    duplicate.append("x-octen-api-key", HeaderValue::from_static("two"));
    let mut empty = HeaderMap::new();
    empty.insert("x-octen-api-key", HeaderValue::from_static(""));
    for headers in [HeaderMap::new(), conflicting, duplicate, empty] {
        let response = call(&server, headers).await;
        assert_eq!(
            response.pointer("/result/isError"),
            Some(&json!(true)),
            "{response}"
        );
    }
    assert!(upstream.requests.lock().await.is_empty());
}
