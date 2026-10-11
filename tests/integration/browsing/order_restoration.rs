use crate::listener::LocalServer;
use alloc::sync::Arc;
use axum::{
    Json, Router,
    extract::State,
    routing::{get, post},
};
use rmcp::serde_json::{Value, json};
use tokio::sync::Mutex;
use web_rs::{
    net::secure_client,
    page::{PageFetcher, fetcher::PageSource, reader::ReaderCredentials},
};
async fn partial_reader(
    State(calls): State<Arc<Mutex<Vec<Value>>>>,
    Json(body): Json<Value>,
) -> Json<Value> {
    let urls = body.get("urls").unwrap().as_array().unwrap();
    let good = urls
        .iter()
        .find(|url| url.as_str().unwrap().ends_with("/b"))
        .unwrap();
    let blocked = urls
        .iter()
        .find(|url| url.as_str().unwrap().ends_with("/blocked"))
        .unwrap();
    let reply = json ! ({ "results" : [{ "url" : good , "text" : "# Reader B" }] , "errors" : [{ "url" : blocked , "error" : "bot_blocked" , "status" : null }] });
    calls.lock().await.push(body);
    Json(reply)
}
#[tokio::test]
async fn page_batch_deduplicates_io_and_preserves_direct_success_reader_failure_and_order() {
    let calls = Arc::new(Mutex::new(Vec::new()));
    let server = LocalServer::start(
        Router::new()
            .route(
                "/direct",
                get(|| async { ([("content-type", "text/markdown")], "# Direct") }),
            )
            .route("/api", post(partial_reader))
            .with_state(Arc::clone(&calls)),
    )
    .await;
    let settings = server.settings();
    let fetcher = PageFetcher::new(
        Arc::new(settings.clone()),
        secure_client(&settings.http, &settings.ssrf).unwrap(),
    );
    let mut urls = ["b", "direct", "blocked", "b", "missing"]
        .map(|path| format!("{}/{path}", server.address))
        .to_vec();
    urls.push("file:///secret".to_owned());
    let pages = fetcher
        .fetch_many_partial(&urls, Some(&ReaderCredentials::TinyFish("key".to_owned())))
        .await;
    assert_eq!(pages.len(), urls.len());
    let expected = [
        Ok((PageSource::Reader, "# Reader B")),
        Ok((PageSource::Direct, "# Direct")),
        Err("bot_blocked"),
        Ok((PageSource::Reader, "# Reader B")),
        Err("no content"),
        Err("HTTP or HTTPS"),
    ];
    for ((url, result), expectation) in urls.iter().zip(pages).zip(expected) {
        match expectation {
            Ok((source, content)) => {
                let page = result.unwrap();
                assert_eq!(&page.url, url);
                assert_eq!(page.source, source);
                assert_eq!(page.markdown.as_ref(), content);
            }
            Err(message) => {
                let error = result.unwrap_err();
                assert!(error.client_message().contains(message), "{error}");
            }
        }
    }
    let captured = calls.lock().await;
    assert_eq!(captured.len(), 1);
    let reader_urls = ["b", "blocked", "missing"].map(|path| format!("{}/{path}", server.address));
    assert_eq!(
        captured.first().unwrap().get("urls"),
        Some(&json!(reader_urls))
    );
    assert_eq!(
        server
            .visits
            .lock()
            .await
            .iter()
            .filter(|visit| visit.uri == "/direct")
            .count(),
        2
    );
}
