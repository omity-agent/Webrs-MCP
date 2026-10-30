use crate::assertions::JsonProbe as _;
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
    let reply = channel :: call (service , "tools/call" , json ! ({ "name" : "open" , "arguments" : { "requests" : [{ "url" : url , "chunk" : 0_u64 } , { "url" : missing , "chunk" : 0_u64 } , { "url" : url , "chunk" : 2_u64 } , { "url" : url , "chunk" : 99_u64 }] } }) ,) . await ;
    assert_eq!(reply.at("/result/isError"), &json!(false), "{reply}");
    let results = reply.list("/result/structuredContent/results");
    assert_eq!(results.len(), 4);
    #[expect(
        clippy::pattern_type_mismatch,
        reason = "Borrowed JSON results are destructured without copying the response envelope."
    )]
    let [first, failed, last, corrected] = results else {
        panic!("expected four ordered open results: {results:?}");
    };
    for (result, chunk, content) in [
        (first, 0_u64, " alpha".repeat(10)),
        (last, 2_u64, " alpha".repeat(8)),
        (corrected, 0_u64, " alpha".repeat(10)),
    ] {
        assert_eq!(result.at("/url"), &json!(url));
        assert_eq!(result.at("/page/chunk"), &json!(chunk));
        assert_eq!(result.at("/page/total_chunks"), &json!(3_u64));
        assert_eq!(result.at("/page/content"), &json!(content));
    }
    assert_eq!(failed.at("/url"), &json!(missing));
    assert!(failed.text("/error").contains("API key"));
    assert!(failed.get("page").is_none());
    let warning = reply.text("/result/structuredContent/warning/0");
    assert!(
        warning.contains("requests[3].chunk") && warning.contains("using 0"),
        "{warning}"
    );
    let text = reply.text("/result/content/0/text");
    assert!(text.starts_with(&format!("<0>\n<URL>\n{url}\n</URL>\n<PAGE>")));
    assert!(text.contains(&format!("</0>\n<1>\n<URL>\n{missing}\n</URL>\n<ERROR>")));
    assert!(text.contains("</ERROR>\n</1>\n<2>"));
    assert!(text.contains("</PAGE>\n</2>\n<3>"));
    assert!(text.ends_with(&format!(
        "</PAGE>\n</3>\n<WARNINGS>\n<WARNING>\n{warning}\n</WARNING>\n</WARNINGS>"
    )));
    assert_eq!(
        server
            .uris()
            .await
            .iter()
            .filter(|uri| *uri == "/page")
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
    let reply = channel :: call (service . clone () , "tools/call" , json ! ({ "name" : "find" , "arguments" : { "requests" : [{ "url" : url , "pattern" : "中文(?=!)" , "snippet_tokens" : 5_u64 } , { "url" : url , "pattern" : "absent" , "snippet_tokens" : 2000_u64 } , { "url" : url , "pattern" : "中文(?=!)" , "snippet_tokens" : 5_u64 }] } }) ,) . await ;
    assert_eq!(reply.at("/result/isError"), &json!(false), "{reply}");
    let pages = reply.list("/result/structuredContent/pages");
    assert_eq!(pages.len(), 3);
    #[expect(
        clippy::pattern_type_mismatch,
        reason = "Borrowed JSON pages are destructured without copying the response envelope."
    )]
    let [found_page, empty_page, repeated_page] = pages else {
        panic!("expected three ordered find results: {pages:?}");
    };
    assert_eq!(repeated_page, found_page);
    assert_eq!(found_page.at("/total_chunks"), &json!(1_u64));
    let matches = found_page.list("/matches");
    assert_eq!(matches.len(), 2);
    for found in matches {
        let snippet = found.text("/snippet");
        assert!(snippet.contains("中文"), "{snippet}");
        assert!(encoder.encode(snippet).len() <= 5, "{snippet}");
        assert_eq!(found.at("/chunk"), &json!(0_u64));
    }
    assert_eq!(empty_page.at("/matches"), &json!([]));
    let warning = reply.text("/result/structuredContent/warning/0");
    assert!(
        warning.contains("requests[1].snippet_tokens") && warning.contains("1000"),
        "{warning}"
    );
    let text = reply.text("/result/content/0/text");
    assert!(text.contains(matches.first().unwrap().text("/snippet")));
    assert!(text.contains(warning) && text.contains("<MATCHES>"));
    server.visits.lock().await.clear();
    let invalid = channel :: call (service , "tools/call" , json ! ({ "name" : "find" , "arguments" : { "requests" : [{ "url" : url , "pattern" : "[" }] } }) ,) . await ;
    assert_eq!(invalid.at("/result/isError"), &json!(true));
    assert_eq!(server.uris().await.len(), 0);
}
