use super::fixture::Listener;
use axum::{
    Router,
    http::{HeaderMap, HeaderValue, StatusCode, header::AUTHORIZATION},
    response::Redirect,
    routing::get,
};
use core::time::Duration;
use web_rs::{config, net::secure_client};
#[tokio::test]
async fn cross_origin_redirect_strips_authorization() {
    let destination = Listener::start(Router::new().route(
        "/",
        get(|headers: HeaderMap| async move {
            if headers.contains_key(AUTHORIZATION) {
                "credential leaked"
            } else {
                "no credentials"
            }
        }),
    ))
    .await;
    let target = format!("{}/", destination.address);
    let source = Listener::start(Router::new().route(
        "/",
        get(move || async move { Redirect::temporary(&target) }),
    ))
    .await;
    let mut settings = config::load_embedded().unwrap();
    settings.ssrf.block_private_networks = false;
    let client = secure_client(&settings.http, &settings.ssrf).unwrap();
    let mut headers = HeaderMap::new();
    headers.insert(AUTHORIZATION, HeaderValue::from_static("Bearer secret"));
    let response = client.get(&source.address, headers, 5.0).await.unwrap();
    assert_eq!(response.body.as_ref(), b"no credentials");
}
#[tokio::test]
async fn not_modified_is_not_a_redirect() {
    let server = Listener::start(
        Router::new()
            .route(
                "/",
                get(|| async { (StatusCode::NOT_MODIFIED, [("location", "/target")]) }),
            )
            .route("/target", get(|| async { "unexpected follow" })),
    )
    .await;
    let mut settings = config::load_embedded().unwrap();
    settings.ssrf.block_private_networks = false;
    let client = secure_client(&settings.http, &settings.ssrf).unwrap();
    let response = client
        .get(&server.address, HeaderMap::new(), 5.0)
        .await
        .unwrap();
    assert_eq!(response.status, StatusCode::NOT_MODIFIED);
}
#[tokio::test]
async fn redirect_chain_shares_one_deadline() {
    let server = Listener::start(
        Router::new()
            .route(
                "/",
                get(|| async {
                    tokio::time::sleep(Duration::from_millis(80)).await;
                    Redirect::temporary("/middle")
                }),
            )
            .route(
                "/middle",
                get(|| async {
                    tokio::time::sleep(Duration::from_millis(80)).await;
                    Redirect::temporary("/end")
                }),
            )
            .route("/end", get(|| async { "done" })),
    )
    .await;
    let mut settings = config::load_embedded().unwrap();
    settings.ssrf.block_private_networks = false;
    let client = secure_client(&settings.http, &settings.ssrf).unwrap();
    let result = client.get(&server.address, HeaderMap::new(), 0.12).await;
    assert!(result.is_err(), "the deadline was reset by a redirect");
}
#[tokio::test]
async fn redirect_cannot_introduce_url_credentials() {
    let server = Listener::start(Router::new().route(
        "/",
        get(|| async { Redirect::temporary("http://secret:password@localhost/") }),
    ))
    .await;
    let mut settings = config::load_embedded().unwrap();
    settings.ssrf.block_private_networks = false;
    let client = secure_client(&settings.http, &settings.ssrf).unwrap();
    let error = client
        .get(&server.address, HeaderMap::new(), 5.0)
        .await
        .unwrap_err();
    assert!(error.client_message().contains("credentials"));
}
#[tokio::test]
async fn original_url_credentials_are_rejected_before_request_building() {
    let server =
        Listener::start(Router::new().route("/", get(|| async { "unexpected request" }))).await;
    let mut settings = config::load_embedded().unwrap();
    settings.ssrf.block_private_networks = false;
    let client = secure_client(&settings.http, &settings.ssrf).unwrap();
    let url = server
        .address
        .replacen("http://", "http://secret:password@", 1);
    let error = client.get(&url, HeaderMap::new(), 5.0).await.unwrap_err();
    assert!(error.client_message().contains("credentials"));
}
