use super::fixture::{Listener, query};
use alloc::sync::Arc;
use axum::{
    Router,
    http::{HeaderMap, StatusCode},
    response::Redirect,
    routing::{get, post},
};
use core::time::Duration;
use futures::{StreamExt as _, stream};
use tokio::sync::Notify;
use web_rs::{
    config,
    net::secure_client,
    search::{SearchClient, SearchCredentials},
};
#[tokio::test]
async fn providers_have_a_response_body_limit() {
    let server =
        Listener::start(Router::new().route("/", post(|| async { "too much content" }))).await;
    let mut settings = config::load_embedded().unwrap();
    settings.ssrf.block_private_networks = false;
    settings.search.exa.endpoint.clone_from(&server.address);
    settings.http.max_response_bytes = 8;
    let http = secure_client(&settings.http, &settings.ssrf).unwrap();
    let search = SearchClient::new(&settings, http);
    let error = search
        .search_many(
            &[query("test")],
            &SearchCredentials::Exa("key".to_owned()),
            true,
        )
        .await
        .err()
        .unwrap();
    assert!(error.client_message().contains("8 byte limit"));
}
#[tokio::test]
async fn api_posts_do_not_follow_redirects() {
    let server = Listener::start(
        Router::new()
            .route("/", post(|| async { Redirect::temporary("/target") }))
            .route("/target", post(|| async { "{\"results\":[]}" })),
    )
    .await;
    let mut settings = config::load_embedded().unwrap();
    settings.ssrf.block_private_networks = false;
    settings.search.exa.endpoint.clone_from(&server.address);
    let http = secure_client(&settings.http, &settings.ssrf).unwrap();
    let search = SearchClient::new(&settings, http);
    let result = search
        .search_many(
            &[query("test")],
            &SearchCredentials::Exa("key".to_owned()),
            true,
        )
        .await;
    assert!(result.err().unwrap().client_message().contains("307"));
}
#[tokio::test]
async fn deadline_includes_queueing_and_body_permits_are_shared() {
    let started = Arc::new(Notify::new());
    let release = Arc::new(Notify::new());
    let entered = Arc::clone(&started);
    let gate = Arc::clone(&release);
    let server = Listener::start(
        Router::new()
            .route(
                "/hold",
                get(move || async move {
                    entered.notify_one();
                    let chunks = stream::once(async move {
                        gate.notified().await;
                        Ok::<_, core::convert::Infallible>("done")
                    });
                    axum::body::Body::from_stream(chunks.boxed())
                }),
            )
            .route(
                "/search",
                post(|| async { (StatusCode::OK, "{\"results\":[]}") }),
            ),
    )
    .await;
    let mut settings = config::load_embedded().unwrap();
    settings.ssrf.block_private_networks = false;
    settings.http.max_concurrent_requests = 1;
    settings.http.timeout_seconds = 0.05_f64;
    settings.search.exa.endpoint = format!("{}/search", server.address);
    let http = secure_client(&settings.http, &settings.ssrf).unwrap();
    let direct = http.clone();
    let url = format!("{}/hold", server.address);
    let holding = tokio::spawn(async move { direct.get(&url, HeaderMap::new(), 5.0).await });
    tokio::time::timeout(Duration::from_secs(5), started.notified())
        .await
        .unwrap();
    let search = SearchClient::new(&settings, http);
    let result = search
        .search_many(
            &[query("test")],
            &SearchCredentials::Exa("key".to_owned()),
            true,
        )
        .await;
    release.notify_one();
    holding.await.unwrap().unwrap();
    assert!(result.err().unwrap().client_message().contains("timed out"));
}
