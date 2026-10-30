use crate::assertions::JsonProbe as _;
use crate::{
    envelope::{STATELESS, request},
    listener::LocalServer,
    process::{Process, renamed_settings, settings_file},
};
use axum::{Router, routing::get};
use rmcp::serde_json::json;
#[tokio::test]
async fn stdio_defaults_to_chunks_and_no_chunking_changes_schema_and_content() {
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
    let mut settings = renamed_settings();
    settings.chunking.chunk_tokens = 5;
    settings.chunking.overlap_ratio = 0.0_f64;
    let file = settings_file(&settings);
    let url = format!("{}/page", origin.address);
    for chunking in [true, false] {
        let arguments: &[&str] = if chunking { &[] } else { &["--no-chunking"] };
        let mut process = Process::start_configured(&file, arguments, &[]);
        let listed = process
            .call(&request(STATELESS, "tools/list", json!({})))
            .await;
        let schema = sonic_rs::to_string(listed.at("/result/tools")).unwrap();
        for field in ["\"chunk\"", "\"total_chunks\""] {
            assert_eq!(schema.contains(field), chunking, "{listed}");
        }
        let input = if chunking {
            json!({ "url": url, "chunk": 1_u64 })
        } else {
            json!({ "url": url })
        };
        let opened = process
            .call(&request(
                STATELESS,
                "tools/call",
                json!({ "name": "browse", "arguments": { "requests": [input] } }),
            ))
            .await;
        assert_eq!(opened.at("/result/isError"), &json!(false));
        let page = if chunking {
            json!({ "chunk": 1_u64, "total_chunks": 5_u64, "content": " alpha".repeat(5) })
        } else {
            json!({ "content": content })
        };
        assert_eq!(
            opened.at("/result/structuredContent"),
            &json!({ "results": [{ "url": url, "page": page }] })
        );
        let text = opened.text("/result/content/0/text");
        assert_eq!(text.contains("<CHUNK>"), chunking);
        assert_eq!(text.contains("<TOTAL_CHUNKS>"), chunking);
        if !chunking {
            let found = process.call(&request(STATELESS, "tools/call", json!({
                "name": "scan",
                "arguments": {"requests": [
                    {"url": url, "pattern": "^ alpha(?: alpha){23}$", "snippet_tokens": 100_u64}
                ]}
            }))).await;
            assert_eq!(
                found.at("/result/structuredContent/pages"),
                &json!([
                    {"matches": [{"snippet": content}]}
                ])
            );
        }
        process.close().await;
    }
}
