use crate::listener::LocalServer;
use alloc::sync::Arc;
use axum::{
    Json, Router,
    extract::Query,
    response::Redirect,
    routing::{get, post},
};
use rmcp::serde_json::json;
use std::collections::HashMap;
use web_rs::{
    config::AppConfig,
    net::secure_client,
    page::{PageFetcher, fetcher::PageSource, reader::ReaderCredentials},
};
fn fetcher(settings: AppConfig) -> PageFetcher {
    let client = secure_client(&settings.http, &settings.ssrf).unwrap();
    PageFetcher::new(Arc::new(settings), client)
}
#[tokio::test]
async fn candidates_follow_the_final_redirect_path_and_preserve_its_query() {
    let server = LocalServer::start(
        Router::new()
            .route("/short", get(|| async { Redirect::temporary("/middle") }))
            .route(
                "/middle",
                get(|| async { Redirect::permanent("/docs/page.html?lang=zh") }),
            )
            .route(
                "/docs/page.html",
                get(|| async { ([("content-type", "text/html")], "<html>Document</html>") }),
            )
            .route(
                "/docs/page.md",
                get(|| async { ([("content-type", "text/markdown")], "# Redirected document") }),
            ),
    )
    .await;
    let original = format!("{}/short", server.address);
    let page = fetcher(server.settings())
        .fetch(&original, None)
        .await
        .unwrap();
    assert_eq!(page.markdown.as_ref(), "# Redirected document");
    assert_eq!(page.url, original);
    assert_eq!(page.source, PageSource::Direct);
    let visits = server.visits.lock().await.clone();
    assert_eq!(
        visits.first().unwrap().headers.get("accept").unwrap(),
        "text/html"
    );
    assert!(
        visits
            .iter()
            .all(|visit| !visit.uri.starts_with("/short.") && !visit.uri.starts_with("/short/"))
    );
    assert!(
        visits
            .iter()
            .all(|visit| !visit.headers.contains_key("range"))
    );
    let destination = visits
        .iter()
        .position(|visit| visit.uri == "/docs/page.html?lang=zh")
        .unwrap();
    let markdown = visits
        .iter()
        .position(|visit| visit.uri == "/docs/page.md?lang=zh")
        .unwrap();
    assert!(destination < markdown);
}
#[tokio::test]
async fn soft_404_candidates_share_one_probe_and_yield_to_the_reader() {
    let server = LocalServer::start(
        Router::new()
            .route(
                "/page",
                get(|| async { ([("content-type", "text/html")], "<html>Page</html>") }),
            )
            .route(
                "/api",
                post(|| async { Json(json ! ({ "data" : { "markdown" : "# Real page" } })) }),
            )
            .fallback(|| async {
                (
                    [("content-type", "text/markdown")],
                    "generated missing page",
                )
            }),
    )
    .await;
    let credentials = ReaderCredentials::Jina("key".to_owned());
    let page = fetcher(server.settings())
        .fetch(&format!("{}/page", server.address), Some(&credentials))
        .await
        .unwrap();
    assert_eq!(page.markdown.as_ref(), "# Real page");
    assert_eq!(page.source, PageSource::Reader);
    let visits = server.visits.lock().await;
    let probes = visits
        .iter()
        .filter(|visit| visit.uri.starts_with("/page.") && visit.uri != "/page.md")
        .count();
    assert_eq!(probes, 1);
    assert_eq!(visits.iter().filter(|visit| visit.uri == "/api").count(), 1);
}
#[tokio::test]
#[expect(
    clippy::literal_string_with_formatting_args,
    reason = "Axum route placeholders describe path captures rather than Rust formatting."
)]
async fn learn_specialization_follows_redirects_without_affecting_excluded_sections() {
    let server = LocalServer::start(
        Router::new()
            .route(
                "/short",
                get(|| async { Redirect::temporary("/en-us/windows/article") }),
            )
            .route(
                "/{locale}/{section}/{page}",
                get(|Query(query): Query<HashMap<String, String>>| async move {
                    if query
                        .get("accept")
                        .is_some_and(|value| value == "text/markdown")
                    {
                        (
                            [("content-type", "text/markdown")],
                            "---\nlayout: Conceptual\ntitle: Learn\n---\n\n# Learn document",
                        )
                    } else {
                        ([("content-type", "text/html")], "<html>Document</html>")
                    }
                }),
            ),
    )
    .await;
    let mut settings = server.settings();
    settings.direct_fetch.microsoft_learn_hosts = vec!["127.0.0.1".to_owned()];
    let client = fetcher(settings);
    let page = client
        .fetch(&format!("{}/short", server.address), None)
        .await
        .unwrap();
    assert_eq!(page.markdown.as_ref(), "# Learn document");
    assert!(
        server
            .uris()
            .await
            .iter()
            .any(|uri| uri == "/en-us/windows/article?accept=text%2Fmarkdown")
    );
    server.visits.lock().await.clear();
    for path in [
        "/en-us/answers/page",
        "/%65n-us/%61nswers/page",
        "/fil-ph/training/page",
        "/sr-latn-rs/shows/page",
    ] {
        let error = client
            .fetch(&format!("{}{path}", server.address), None)
            .await
            .unwrap_err();
        assert!(error.client_message().contains("API key"), "{error}");
    }
    assert!(
        server
            .uris()
            .await
            .iter()
            .all(|uri| !uri.contains("accept="))
    );
}
