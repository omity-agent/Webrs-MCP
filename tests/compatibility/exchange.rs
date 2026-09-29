use super::wire::{Endpoint, LEGACY, MODERN, assert_tools, initialize, request, response};
use reqwest::StatusCode;
use rmcp::serde_json::json;
#[tokio::test]
async fn modern_http_needs_no_handshake_and_concurrent_versions_do_not_leak() {
    let endpoint = Endpoint::new(true).await;
    let modern = request(MODERN, "tools/list", json!({}));
    let direct = response(endpoint.post(&modern, Some(MODERN)), StatusCode::OK).await;
    assert_tools(&direct, MODERN, false);
    let init = response(endpoint.post(&initialize(LEGACY), None), StatusCode::OK).await;
    assert_eq!(
        init.pointer("/result/protocolVersion"),
        Some(&json!(LEGACY))
    );
    let legacy = request(LEGACY, "tools/list", json!({}));
    let (old, new) = tokio::join!(
        response(endpoint.post(&legacy, Some(LEGACY)), StatusCode::OK),
        response(endpoint.post(&modern, Some(MODERN)), StatusCode::OK),
    );
    assert_tools(&old, LEGACY, false);
    assert_tools(&new, MODERN, false);
}
#[tokio::test]
async fn http_serves_both_versions_with_json_and_sse() {
    for json_response in [true, false] {
        let endpoint = Endpoint::new(json_response).await;
        let init = response(endpoint.post(&initialize(LEGACY), None), StatusCode::OK).await;
        assert_eq!(
            init.pointer("/result/protocolVersion"),
            Some(&json!(LEGACY))
        );
        assert_eq!(init.pointer("/result/serverInfo/name"), Some(&json!("web")));
        assert!(init.pointer("/result/capabilities/tools").is_some());
        let notification = json ! ({ "jsonrpc" : "2.0" , "method" : "notifications/initialized" });
        let accepted = endpoint
            .post(&notification, Some(LEGACY))
            .send()
            .await
            .unwrap();
        assert_eq!(accepted.status(), StatusCode::ACCEPTED);
        assert!(accepted.bytes().await.unwrap().is_empty());
        for version in [MODERN, LEGACY, MODERN, LEGACY] {
            let body = request(version, "tools/list", json!({}));
            let result = response(endpoint.post(&body, Some(version)), StatusCode::OK).await;
            assert_tools(&result, version, false);
        }
        let discover = request(MODERN, "server/discover", json!({}));
        let info = response(endpoint.post(&discover, Some(MODERN)), StatusCode::OK).await;
        assert_eq!(
            info.pointer("/result/supportedVersions"),
            Some(&json!([LEGACY, MODERN]))
        );
    }
}
#[tokio::test]
async fn initialization_negotiates_only_a_supported_handshake_revision() {
    let endpoint = Endpoint::new(true).await;
    for proposed in [LEGACY, MODERN, "2025-11-25", "1900-01-01"] {
        let result = response(endpoint.post(&initialize(proposed), None), StatusCode::OK).await;
        assert_eq!(
            result.pointer("/result/protocolVersion"),
            Some(&json!(LEGACY))
        );
    }
}
#[tokio::test]
async fn ping_and_http_methods_respect_the_selected_protocol() {
    let endpoint = Endpoint::new(true).await;
    let ping = request(LEGACY, "ping", json!({}));
    let pong = response(endpoint.post(&ping, Some(LEGACY)), StatusCode::OK).await;
    assert_eq!(pong.get("result"), Some(&json!({})));
    let removed = request(MODERN, "ping", json!({}));
    let error = response(endpoint.post(&removed, Some(MODERN)), StatusCode::NOT_FOUND).await;
    assert_eq!(error.pointer("/error/code"), Some(&json!(-32601_i64)));
    for version in [LEGACY, MODERN] {
        for method in [reqwest::Method::GET, reqwest::Method::DELETE] {
            let reply = endpoint
                .client
                .request(method, &endpoint.url)
                .header("mcp-protocol-version", version)
                .send()
                .await
                .unwrap();
            assert_eq!(reply.status(), StatusCode::METHOD_NOT_ALLOWED);
            assert_eq!(reply.headers().get("allow").unwrap(), "POST");
        }
    }
}
