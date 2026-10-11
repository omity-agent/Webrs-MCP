use crate::{
    envelope::{HANDSHAKE, STATELESS, request},
    rpc::{Endpoint, response},
};
use reqwest::StatusCode;
use rmcp::serde_json::{Value, json};
use web_rs::config;
#[tokio::test]
async fn stateless_requests_require_metadata_and_matching_routing_headers() {
    let endpoint = Endpoint::configured(config::load_embedded().unwrap()).await;
    for missing in [
        "_meta",
        "io.modelcontextprotocol/protocolVersion",
        "io.modelcontextprotocol/clientCapabilities",
    ] {
        let mut body = request(STATELESS, "tools/list", json!({}));
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
        let reply = response(endpoint.post(&body, STATELESS), StatusCode::BAD_REQUEST).await;
        assert_eq!(reply.pointer("/error/code"), Some(&json!(-32602_i64)));
        assert!(reply.get("result").is_none());
    }
    let call = request(
        STATELESS,
        "tools/call",
        json ! ({ "name" : "open" , "arguments" : { } }),
    );
    for (header, replacement) in [
        ("mcp-protocol-version", Some(HANDSHAKE)),
        ("mcp-method", Some("tools/list")),
        ("mcp-name", Some("find")),
        ("mcp-protocol-version", None),
        ("mcp-method", None),
        ("mcp-name", None),
    ] {
        let mut built = endpoint.post(&call, STATELESS).build().unwrap();
        if let Some(value) = replacement {
            built.headers_mut().insert(header, value.parse().unwrap());
        } else {
            built.headers_mut().remove(header);
        }
        let reply = endpoint.client.execute(built).await.unwrap();
        assert_eq!(reply.status(), StatusCode::BAD_REQUEST);
        let body: Value = reply.json().await.unwrap();
        assert_eq!(
            body.pointer("/error/code"),
            Some(&json!(-32020_i64)),
            "{body}"
        );
        assert!(body.get("result").is_none());
    }
}
#[tokio::test]
async fn unsupported_protocol_reports_supported_revisions_and_http_methods_are_post_only() {
    let endpoint = Endpoint::configured(config::load_embedded().unwrap()).await;
    let unknown = "2099-01-01";
    let mut body = request(STATELESS, "tools/list", json!({}));
    *body
        .pointer_mut("/params/_meta/io.modelcontextprotocol~1protocolVersion")
        .unwrap() = json!(unknown);
    let reply = response(
        endpoint
            .post(&body, unknown)
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
        Some(&json!([HANDSHAKE, STATELESS]))
    );
    for method in [reqwest::Method::GET, reqwest::Method::DELETE] {
        let response = endpoint
            .client
            .request(method, format!("{}/mcp", endpoint.server.address))
            .header("mcp-protocol-version", STATELESS)
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
        assert_eq!(response.headers().get("allow").unwrap(), "POST");
    }
}
