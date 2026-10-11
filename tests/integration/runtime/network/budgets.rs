use crate::{listener::LocalServer, upstream::query};
use alloc::sync::Arc;
use axum::{
    Router,
    body::Body,
    http::HeaderMap,
    response::Redirect,
    routing::{get, post},
};
use bytes::Bytes;
use core::{convert::Infallible, time::Duration};
use futures::{StreamExt as _, stream};
use tokio::sync::Notify;
use web_rs::{
    net::secure_client,
    search::{SearchClient, SearchCredentials},
};
#[tokio::test]
async fn declared_and_streamed_bodies_enforce_exact_limits_but_resolution_skips_content() {
    let server = LocalServer::start(
        Router::new()
            .route("/declared", get(|| async { "abcdef" }))
            .route(
                "/streamed",
                get(|| async {
                    Body::from_stream(stream::iter([
                        Ok::<_, Infallible>(Bytes::from_static(b"abc")),
                        Ok(Bytes::from_static(b"def")),
                    ]))
                }),
            ),
    )
    .await;
    let mut settings = server.settings();
    settings.http.max_response_bytes = 6;
    let client = secure_client(&settings.http, &settings.ssrf).unwrap();
    for path in ["/declared", "/streamed"] {
        let url = format!("{}{path}", server.address);
        assert_eq!(
            client
                .get_with_body_limit(&url, HeaderMap::new(), 5.0, 6)
                .await
                .unwrap()
                .body
                .as_ref(),
            b"abcdef"
        );
        let error = client
            .get_with_body_limit(&url, HeaderMap::new(), 5.0, 5)
            .await
            .unwrap_err();
        assert!(error.client_message().contains("5 byte limit"), "{error}");
    }
    settings.http.max_response_bytes = 1;
    let resolving = secure_client(&settings.http, &settings.ssrf).unwrap();
    let url = format!("{}/declared", server.address);
    assert_eq!(
        resolving.resolve_url(&url, 5.0).await.unwrap().as_str(),
        url
    );
    assert!(
        resolving
            .get(&url, HeaderMap::new(), 5.0)
            .await
            .unwrap_err()
            .client_message()
            .contains("1 byte limit")
    );
}
#[tokio::test]
async fn provider_posts_obey_body_limits_and_never_replay_across_redirects() {
    let server = LocalServer::start(
        Router::new()
            .route("/large", post(|| async { "too much content" }))
            .route(
                "/redirect",
                post(|| async { Redirect::temporary("/target") }),
            )
            .route("/target", post(|| async { "{\"results\":[]}" })),
    )
    .await;
    for (path, limit, expected) in [("/large", 8, "8 byte limit"), ("/redirect", 1024, "307")] {
        let mut settings = server.settings();
        settings.search.exa.endpoint = format!("{}{path}", server.address);
        settings.http.max_response_bytes = limit;
        let search = SearchClient::new(
            &settings,
            secure_client(&settings.http, &settings.ssrf).unwrap(),
        );
        let error = search
            .search_many(
                &[query("test")],
                &SearchCredentials::Exa("key".to_owned()),
                true,
            )
            .await
            .err()
            .unwrap();
        assert!(error.client_message().contains(expected), "{error}");
    }
    assert!(
        server
            .visits
            .lock()
            .await
            .iter()
            .all(|visit| visit.uri != "/target")
    );
}
#[tokio::test]
async fn cloned_http_clients_share_body_permits_and_queue_time_counts_toward_deadline() {
    let started = Arc::new(Notify::new());
    let release = Arc::new(Notify::new());
    let entered = Arc::clone(&started);
    let gate = Arc::clone(&release);
    let server = LocalServer::start(
        Router::new()
            .route(
                "/hold",
                get(move || async move {
                    entered.notify_one();
                    let chunks = stream::once(async move {
                        gate.notified().await;
                        Ok::<_, Infallible>("done")
                    });
                    Body::from_stream(chunks.boxed())
                }),
            )
            .route("/api", post(|| async { "{\"results\":[]}" })),
    )
    .await;
    let mut settings = server.settings();
    settings.http.max_concurrent_requests = 1;
    settings.http.timeout_seconds = 0.1_f64;
    let client = secure_client(&settings.http, &settings.ssrf).unwrap();
    let direct = client.clone();
    let url = format!("{}/hold", server.address);
    let holding = tokio::spawn(async move { direct.get(&url, HeaderMap::new(), 5.0).await });
    tokio::time::timeout(Duration::from_secs(5), started.notified())
        .await
        .unwrap();
    let search = SearchClient::new(&settings, client);
    let key = SearchCredentials::Exa("key".to_owned());
    let queued = search.search_many(&[query("queued")], &key, true).await;
    release.notify_one();
    assert_eq!(holding.await.unwrap().unwrap().body.as_ref(), b"done");
    assert!(queued.err().unwrap().client_message().contains("timed out"));
    assert!(
        server
            .visits
            .lock()
            .await
            .iter()
            .all(|visit| visit.uri != "/api")
    );
    search
        .search_many(&[query("recovered")], &key, true)
        .await
        .unwrap();
    assert_eq!(
        server
            .visits
            .lock()
            .await
            .iter()
            .filter(|visit| visit.uri == "/api")
            .count(),
        1
    );
}
