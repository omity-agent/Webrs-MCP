use crate::{channel, listener::LocalServer};
use axum::{Router, routing::get};
use rmcp::serde_json::json;
use web_rs::mcp::tools::ToolService;
#[tokio::test]
async fn open_preserves_overlap_order_partial_failures_and_chunk_correction_warnings() {
    let server = LocalServer::start(Router::new().route(
        "/page",
        get(|| async { ([("content-type", "text/markdown")], " alpha".repeat(24)) }),
    ))
    .await;
    let mut settings = server.settings();
    settings.chunking.chunk_tokens = 10;
    settings.chunking.overlap_ratio = 0.2_f64;
    let service = ToolService::new(settings).unwrap();
    let url = format!("{}/page", server.address);
    let missing = format!("{}/missing", server.address);
    let reply = channel :: call (service , "tools/call" , json ! ({ "name" : "open" , "arguments" : { "requests" : [{ "url" : url , "chunk" : 0_u64 } , { "url" : url , "chunk" : 2_u64 } , { "url" : url , "chunk" : 99_u64 } , { "url" : missing , "chunk" : 0_u64 }] } }) ,) . await ;
    assert_eq!(
        reply.pointer("/result/isError"),
        Some(&json!(false)),
        "{reply}"
    );
    let results = reply
        .pointer("/result/structuredContent/results")
        .unwrap()
        .as_array()
        .unwrap();
    assert_eq!(results.len(), 4);
    #[expect(
        clippy::pattern_type_mismatch,
        reason = "Borrowed JSON results are destructured without copying the response envelope."
    )]
    let [first, last, corrected, failed] = results.as_slice() else {
        panic!("expected four ordered open results: {results:?}");
    };
    for (result, chunk, content) in [
        (first, 0_u64, " alpha".repeat(10)),
        (last, 2_u64, " alpha".repeat(8)),
        (corrected, 0_u64, " alpha".repeat(10)),
    ] {
        assert_eq!(result.get("url"), Some(&json!(url)));
        assert_eq!(result.pointer("/page/chunk"), Some(&json!(chunk)));
        assert_eq!(result.pointer("/page/total_chunks"), Some(&json!(3_u64)));
        assert_eq!(result.pointer("/page/content"), Some(&json!(content)));
    }
    assert_eq!(failed.get("url"), Some(&json!(missing)));
    assert!(
        failed
            .get("error")
            .unwrap()
            .as_str()
            .unwrap()
            .contains("API key")
    );
    assert!(failed.get("page").is_none());
    let warning = reply
        .pointer("/result/structuredContent/warning/0")
        .unwrap()
        .as_str()
        .unwrap();
    assert!(
        warning.contains("requests[2].chunk") && warning.contains("using 0"),
        "{warning}"
    );
    let text = reply
        .pointer("/result/content/0/text")
        .unwrap()
        .as_str()
        .unwrap();
    assert!(text.contains("<ERROR>") && text.contains(warning));
    assert_eq!(
        server
            .visits
            .lock()
            .await
            .iter()
            .filter(|visit| visit.uri == "/page")
            .count(),
        2
    );
}
#[tokio::test]
async fn find_honors_unicode_regex_snippet_and_match_limits_and_validates_before_fetching() {
    let server = LocalServer::start(Router::new().route(
        "/page",
        get(|| async {
            (
                [("content-type", "text/markdown")],
                "before 中文! after ".repeat(8),
            )
        }),
    ))
    .await;
    let mut settings = server.settings();
    settings.chunking.chunk_tokens = 1000;
    settings.find.max_matches_per_page = 2;
    let encoder = tiktoken::get_encoding(&settings.chunking.tokenizer).unwrap();
    let service = ToolService::new(settings).unwrap();
    let url = format!("{}/page", server.address);
    let reply = channel :: call (service . clone () , "tools/call" , json ! ({ "name" : "find" , "arguments" : { "requests" : [{ "url" : url , "pattern" : "中文(?=!)" , "snippet_tokens" : 5_u64 } , { "url" : url , "pattern" : "absent" , "snippet_tokens" : 2000_u64 }] } }) ,) . await ;
    assert_eq!(
        reply.pointer("/result/isError"),
        Some(&json!(false)),
        "{reply}"
    );
    let pages = reply
        .pointer("/result/structuredContent/pages")
        .unwrap()
        .as_array()
        .unwrap();
    assert_eq!(pages.len(), 2);
    #[expect(
        clippy::pattern_type_mismatch,
        reason = "Borrowed JSON pages are destructured without copying the response envelope."
    )]
    let [found_page, empty_page] = pages.as_slice() else {
        panic!("expected two ordered find results: {pages:?}");
    };
    assert_eq!(found_page.get("total_chunks"), Some(&json!(1_u64)));
    let matches = found_page.get("matches").unwrap().as_array().unwrap();
    assert_eq!(matches.len(), 2);
    for found in matches {
        let snippet = found.get("snippet").unwrap().as_str().unwrap();
        assert!(snippet.contains("中文"), "{snippet}");
        assert!(encoder.encode(snippet).len() <= 5, "{snippet}");
        assert_eq!(found.get("chunk"), Some(&json!(0_u64)));
    }
    assert_eq!(empty_page.get("matches"), Some(&json!([])));
    let warning = reply
        .pointer("/result/structuredContent/warning/0")
        .unwrap()
        .as_str()
        .unwrap();
    assert!(
        warning.contains("requests[1].snippet_tokens") && warning.contains("1000"),
        "{warning}"
    );
    assert!(
        reply
            .pointer("/result/content/0/text")
            .unwrap()
            .as_str()
            .unwrap()
            .contains("<MATCHES>\n\n</MATCHES>")
    );
    server.visits.lock().await.clear();
    let invalid = channel :: call (service , "tools/call" , json ! ({ "name" : "find" , "arguments" : { "requests" : [{ "url" : url , "pattern" : "[" }] } }) ,) . await ;
    assert_eq!(invalid.pointer("/result/isError"), Some(&json!(true)));
    assert!(server.visits.lock().await.is_empty());
}
