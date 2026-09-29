use super::wire::{Endpoint, LEGACY, MODERN, initialize, request, response};
use reqwest::StatusCode;
use rmcp::serde_json::{Value, json};
#[tokio::test]
async fn modern_metadata_remains_required_after_legacy_support_is_enabled() {
    let endpoint = Endpoint::new(true).await;
    for missing in [
        "_meta",
        "io.modelcontextprotocol/protocolVersion",
        "io.modelcontextprotocol/clientCapabilities",
    ] {
        let mut body = request(MODERN, "tools/list", json!({}));
        let params = body.get_mut("params").unwrap().as_object_mut().unwrap();
        if missing == "_meta" {
            params.remove(missing);
        } else {
            params
                .get_mut("_meta")
                .unwrap()
                .as_object_mut()
                .unwrap()
                .remove(missing);
        }
        let reply = response(endpoint.post(&body, Some(MODERN)), StatusCode::BAD_REQUEST).await;
        assert_eq!(reply.pointer("/error/code"), Some(&json!(-32602_i64)));
    }
    for version in [LEGACY, MODERN] {
        let body = request(version, "tools/list", json!({}));
        let reply = response(endpoint.post(&body, None), StatusCode::BAD_REQUEST).await;
        assert_eq!(reply.pointer("/error/code"), Some(&json!(-32020_i64)));
    }
}
#[tokio::test]
async fn conflicting_and_missing_routing_headers_are_rejected() {
    let endpoint = Endpoint::new(true).await;
    let body = request(
        MODERN,
        "tools/call",
        json ! ({ "name" : "open" , "arguments" : { } }),
    );
    for (header, value) in [
        ("mcp-protocol-version", LEGACY),
        ("mcp-method", "tools/list"),
        ("mcp-name", "find"),
    ] {
        let mut built = endpoint.post(&body, Some(MODERN)).build().unwrap();
        built.headers_mut().insert(header, value.parse().unwrap());
        let reply = endpoint.client.execute(built).await.unwrap();
        assert_eq!(reply.status(), StatusCode::BAD_REQUEST);
        let message: Value = reply.json().await.unwrap();
        assert_eq!(message.pointer("/error/code"), Some(&json!(-32020_i64)));
    }
    for missing in ["mcp-method", "mcp-name"] {
        let mut built = endpoint.post(&body, Some(MODERN)).build().unwrap();
        built.headers_mut().remove(missing);
        assert_eq!(
            endpoint.client.execute(built).await.unwrap().status(),
            StatusCode::BAD_REQUEST
        );
    }
    assert_eq!(
        endpoint
            .post(&initialize(LEGACY), Some(MODERN))
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::BAD_REQUEST
    );
}
#[tokio::test]
async fn unsupported_versions_report_the_exact_supported_set() {
    let endpoint = Endpoint::new(true).await;
    for unknown in ["1900-01-01", "2025-11-25", "2099-01-01"] {
        let mut body = request(MODERN, "tools/list", json!({}));
        *body
            .pointer_mut("/params/_meta/io.modelcontextprotocol~1protocolVersion")
            .unwrap() = json!(unknown);
        let reply = response(
            endpoint
                .post(&body, Some(unknown))
                .header("mcp-method", "tools/list"),
            StatusCode::BAD_REQUEST,
        )
        .await;
        assert_eq!(reply.pointer("/error/code"), Some(&json!(-32022_i64)));
        assert_eq!(
            reply.pointer("/error/data/requested"),
            Some(&json!(unknown))
        );
        assert_eq!(
            reply.pointer("/error/data/supported"),
            Some(&json!([LEGACY, MODERN]))
        );
    }
    let unsupported = request("2025-11-25", "tools/list", json!({}));
    assert_eq!(
        endpoint
            .post(&unsupported, Some("2025-11-25"))
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::BAD_REQUEST
    );
}
