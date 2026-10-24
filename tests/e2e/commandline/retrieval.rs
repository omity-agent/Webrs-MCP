use crate::{
    listener::LocalServer,
    process::{PAGE, assert_success, invoke, renamed_settings, settings_file, stdout},
};
use axum::{Router, routing::get};
use rmcp::serde_json::{Value, json};
#[tokio::test]
async fn configured_open_and_find_deliver_real_local_content_with_chunk_and_regex_parameters() {
    let server = LocalServer::start(Router::new().route(
        "/page",
        get(|| async { ([("content-type", "text/markdown")], PAGE) }),
    ))
    .await;
    let file = settings_file(&renamed_settings());
    let url = format!("{}/page", server.address);
    let opened = invoke(
        file.path(),
        &["browse", "--url", &url, "--chunk", "99", "--format", "json"],
    )
    .await;
    assert_success(&opened);
    let response: Value = sonic_rs::from_str(stdout(&opened)).unwrap();
    assert_eq!(
        response.pointer("/results/0/page/content"),
        Some(&json!(PAGE))
    );
    assert_eq!(
        response.pointer("/results/0/page/chunk"),
        Some(&json!(0_u64))
    );
    assert_eq!(
        response.pointer("/results/0/page/total_chunks"),
        Some(&json!(1_u64))
    );
    assert!(
        response
            .pointer("/warning/0")
            .unwrap()
            .as_str()
            .unwrap()
            .contains("using 0")
    );
    let found = invoke(
        file.path(),
        &[
            "scan",
            "--url",
            &url,
            "--pattern",
            "nee[d]le(?= is)",
            "--snippet-tokens",
            "20",
        ],
    )
    .await;
    assert_success(&found);
    let text = stdout(&found);
    assert!(text.starts_with("<0>\n<TOTAL_CHUNKS>\n1\n</TOTAL_CHUNKS>\n<MATCHES>\n<0>\n"));
    assert!(text.contains("<CHUNK>\n0\n</CHUNK>"));
    assert!(text.contains("<SNIPPET>") && text.contains("needle is here"));
    assert!(text.ends_with("</SNIPPET>\n</0>\n</MATCHES>\n</0>\n"));
    assert_eq!(
        server
            .visits
            .lock()
            .await
            .iter()
            .filter(|visit| visit.uri == "/page")
            .count(),
        4
    );
    assert!(
        server
            .visits
            .lock()
            .await
            .iter()
            .all(|visit| !visit.headers.contains_key("range"))
    );
}
