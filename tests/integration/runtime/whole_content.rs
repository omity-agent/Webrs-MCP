use crate::{
    envelope::{STATELESS, request},
    listener::LocalServer,
    rpc::{Endpoint, response},
};
use axum::{Router, routing::get};
use reqwest::StatusCode;
use rmcp::serde_json::json;
#[tokio::test]
async fn full_page_http_calls_preserve_order_errors_and_matches_across_token_boundaries() {
    let content = " alpha".repeat(24);
    let served = content.clone();
    let origin = LocalServer::start(Router::new().route(
        "/page",
        get(move || {
            let body = served.clone();
            async move { ([("content-type", "text/markdown")], body) }
        }),
    ))
    .await;
    let mut settings = origin.settings();
    settings.chunking.enabled = false;
    settings.chunking.chunk_tokens = 5;
    settings.chunking.overlap_ratio = 0.0_f64;
    let endpoint = Endpoint::configured(settings).await;
    let url = format!("{}/page", origin.address);
    let missing = format!("{}/missing", origin.address);
    let opened = response(
        endpoint.post(
            &request(
                STATELESS,
                "tools/call",
                json!({
                    "name": "open",
                    "arguments": {
                        "requests": [{"url": url}, {"url": missing}, {"url": url}]
                    }
                }),
            ),
            STATELESS,
        ),
        StatusCode::OK,
    )
    .await;
    assert_eq!(opened.pointer("/result/isError"), Some(&json!(false)));
    let results = opened
        .pointer("/result/structuredContent/results")
        .unwrap()
        .as_array()
        .unwrap();
    let expected = json!({ "url": url, "page": { "content": content } });
    assert_eq!(results.first(), Some(&expected));
    assert_eq!(results.get(2), Some(&expected));
    let failed = opened
        .pointer("/result/structuredContent/results/1")
        .unwrap();
    assert_eq!(failed.get("url"), Some(&json!(missing)));
    assert!(
        failed
            .get("error")
            .unwrap()
            .as_str()
            .unwrap()
            .contains("API key")
    );
    assert!(
        opened
            .pointer("/result/structuredContent/warning")
            .is_none()
    );
    let text = opened
        .pointer("/result/content/0/text")
        .unwrap()
        .as_str()
        .unwrap();
    assert!(text.contains(&content) && !text.contains("<CHUNK>"));
    assert!(!text.contains("<TOTAL_CHUNKS>"));
    let found = response(
        endpoint.post(
            &request(
                STATELESS,
                "tools/call",
                json!({
                    "name": "find",
                    "arguments": {
                        "requests": [
                            {"url": url, "pattern": "^ alpha(?: alpha){23}$", "snippet_tokens": 100_u64},
                            {"url": url, "pattern": "absent"}
                        ]
                    }
                }),
            ),
            STATELESS,
        ),
        StatusCode::OK,
    )
    .await;
    assert_eq!(found.pointer("/result/isError"), Some(&json!(false)));
    assert_eq!(
        found.pointer("/result/structuredContent"),
        Some(&json!({
            "pages": [{"matches": [{"snippet": content}]}, {"matches": []}]
        }))
    );
    assert_eq!(
        found.pointer("/result/content/0/text"),
        Some(&json!(format!(
            "<0>\n<MATCHES>\n<0>\n<SNIPPET>\n{content}\n</SNIPPET>\n</0>\n</MATCHES>\n</0>\n\
             <1>\n<MATCHES>\n\n</MATCHES>\n</1>"
        )))
    );
}
