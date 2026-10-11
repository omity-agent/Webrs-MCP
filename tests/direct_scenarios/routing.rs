use super::environment::Website;
use axum::{Router, response::Redirect, routing::get};
use web_rs::net::secure_client;
use web_rs::page::fetcher::PageSource;
#[tokio::test]
async fn markdown_candidates_are_built_from_the_redirect_destination() {
    let website = Website::start(
        Router::new()
            .route("/short", get(|| async { Redirect::temporary("/middle") }))
            .route(
                "/middle",
                get(|| async { Redirect::permanent("/docs/page") }),
            )
            .route(
                "/docs/page",
                get(|| async { ([("content-type", "text/html")], "<html>Document</html>") }),
            )
            .route(
                "/docs/page.md",
                get(|| async { ([("content-type", "text/markdown")], "# Redirected document") }),
            ),
    )
    .await;
    let settings = website.settings();
    let original_url = format!("{}/short", website.address);
    let page = website
        .fetcher(&settings)
        .fetch(&original_url, None)
        .await
        .unwrap();
    assert_eq!(page.markdown.as_ref(), "# Redirected document");
    assert_eq!(page.url, original_url);
    assert_eq!(page.source, PageSource::Direct);
    let visits = website.visits.lock().await;
    assert_eq!(visits.first().unwrap().accept, "text/html");
    assert!(
        visits.iter().all(|visit| {
            !visit.uri.starts_with("/short.") && !visit.uri.starts_with("/short/")
        }),
        "candidate paths were generated before redirection: {visits:?}"
    );
    let destination = visits
        .iter()
        .position(|visit| visit.uri == "/docs/page")
        .unwrap();
    let markdown = visits
        .iter()
        .position(|visit| visit.uri == "/docs/page.md")
        .unwrap();
    assert!(destination < markdown);
    drop(visits);
}
#[tokio::test]
async fn generated_missing_pages_are_rejected_and_probe_is_shared() {
    let website = Website::start(
        Router::new()
            .route(
                "/page",
                get(|| async { ([("content-type", "text/html")], "<html>Page</html>") }),
            )
            .fallback(|| async {
                (
                    [("content-type", "text/markdown")],
                    "generated missing page",
                )
            }),
    )
    .await;
    let settings = website.settings();
    let result = website
        .fetcher(&settings)
        .fetch(&format!("{}/page", website.address), None)
        .await;
    assert!(result.is_err(), "a generated missing page was accepted");
    let visits = website.visits.lock().await;
    let probes = visits
        .iter()
        .filter(|visit| visit.uri.starts_with("/page.") && visit.uri != "/page.md")
        .count();
    assert_eq!(probes, 1, "missing-page probe was fetched repeatedly");
    drop(visits);
}
#[tokio::test]
async fn redirect_resolution_does_not_download_the_destination_body() {
    let website =
        Website::start(Router::new().route("/large", get(|| async { "x".repeat(4096) }))).await;
    let mut settings = website.settings();
    settings.http.max_response_bytes = 16;
    let client = secure_client(&settings.http, &settings.ssrf).unwrap();
    let url = format!("{}/large", website.address);
    assert_eq!(client.resolve_url(&url, 5.0).await.unwrap().as_str(), url);
    let error = client
        .get(&url, axum::http::HeaderMap::new(), 5.0)
        .await
        .unwrap_err();
    assert!(error.client_message().contains("16 byte limit"));
}
#[tokio::test]
async fn redirect_resolution_keeps_the_hop_limit_and_credential_guard() {
    let website = Website::start(
        Router::new()
            .route("/loop", get(|| async { Redirect::temporary("/loop") }))
            .route(
                "/credentials",
                get(|| async { Redirect::temporary("http://secret:password@localhost/") }),
            ),
    )
    .await;
    let mut settings = website.settings();
    settings.http.max_redirects = 2;
    let client = secure_client(&settings.http, &settings.ssrf).unwrap();
    for (path, expected) in [
        ("/loop", "Too many redirects"),
        ("/credentials", "credentials"),
    ] {
        let error = client
            .resolve_url(&format!("{}{path}", website.address), 5.0)
            .await
            .unwrap_err();
        assert!(error.client_message().contains(expected));
    }
}
