use crate::listener::LocalServer;
use axum::{
    Router,
    http::{HeaderMap, HeaderValue, StatusCode, header::AUTHORIZATION},
    response::Redirect,
    routing::get,
};
use core::time::Duration;
use web_rs::net::secure_client;
#[tokio::test]
async fn redirect_credentials_are_preserved_only_within_the_same_origin() {
    let destination =
        LocalServer::start(Router::new().route("/", get(|| async { "destination" }))).await;
    let target = destination.address.clone();
    let source = LocalServer::start(
        Router::new()
            .route("/same", get(|| async { Redirect::temporary("/final") }))
            .route("/final", get(|| async { "same origin" }))
            .route(
                "/cross",
                get(move || async move { Redirect::temporary(&target) }),
            ),
    )
    .await;
    let settings = source.settings();
    let client = secure_client(&settings.http, &settings.ssrf).unwrap();
    let mut headers = HeaderMap::new();
    headers.insert(AUTHORIZATION, HeaderValue::from_static("Bearer secret"));
    for path in ["/same", "/cross"] {
        client
            .get(&format!("{}{path}", source.address), headers.clone(), 5.0)
            .await
            .unwrap();
    }
    let visits = source.visits.lock().await.clone();
    let same = visits.iter().find(|visit| visit.uri == "/final").unwrap();
    assert_eq!(same.headers.get(AUTHORIZATION).unwrap(), "Bearer secret");
    let crossed = destination.visits.lock().await.clone();
    assert_eq!(crossed.len(), 1);
    assert!(!crossed.first().unwrap().headers.contains_key(AUTHORIZATION));
}
#[tokio::test]
async fn redirects_and_resolution_revalidate_targets_and_enforce_hop_limits() {
    let server = LocalServer::start(
        Router::new()
            .route("/loop", get(|| async { Redirect::temporary("/loop") }))
            .route(
                "/credentials",
                get(|| async { Redirect::temporary("http://secret:password@localhost/") }),
            )
            .route(
                "/local",
                get(|| async { Redirect::temporary("http://localhost/") }),
            )
            .route(
                "/unchanged",
                get(|| async { (StatusCode::NOT_MODIFIED, [("location", "/target")]) }),
            )
            .route("/target", get(|| async { "unexpected request" })),
    )
    .await;
    let mut settings = server.settings();
    settings.ssrf.block_local_hostnames = true;
    settings.http.max_redirects = 2;
    let client = secure_client(&settings.http, &settings.ssrf).unwrap();
    for (path, expected) in [
        ("/loop", "Too many redirects"),
        ("/credentials", "credentials"),
        ("/local", "blocked"),
    ] {
        let url = format!("{}{path}", server.address);
        let fetched = client.get(&url, HeaderMap::new(), 5.0).await.unwrap_err();
        let resolved = client.resolve_url(&url, 5.0).await.unwrap_err();
        assert!(fetched.client_message().contains(expected), "{fetched}");
        assert!(resolved.client_message().contains(expected), "{resolved}");
    }
    let credential_url = server
        .address
        .replacen("http://", "http://secret:password@", 1);
    assert!(
        client
            .get(&credential_url, HeaderMap::new(), 5.0)
            .await
            .unwrap_err()
            .client_message()
            .contains("credentials")
    );
    let unchanged = client
        .get(
            &format!("{}/unchanged", server.address),
            HeaderMap::new(),
            5.0,
        )
        .await
        .unwrap();
    assert_eq!(unchanged.status, StatusCode::NOT_MODIFIED);
    assert!(
        server
            .visits
            .lock()
            .await
            .iter()
            .all(|visit| visit.uri != "/target" && visit.uri != "/")
    );
}
#[tokio::test]
async fn redirect_chain_uses_one_deadline_instead_of_one_timeout_per_hop() {
    let server = LocalServer::start(
        Router::new()
            .route(
                "/",
                get(|| async {
                    tokio::time::sleep(Duration::from_millis(150)).await;
                    Redirect::temporary("/middle")
                }),
            )
            .route(
                "/middle",
                get(|| async {
                    tokio::time::sleep(Duration::from_millis(150)).await;
                    Redirect::temporary("/end")
                }),
            )
            .route("/end", get(|| async { "done" })),
    )
    .await;
    let settings = server.settings();
    let client = secure_client(&settings.http, &settings.ssrf).unwrap();
    let error = client
        .get(&server.address, HeaderMap::new(), 0.25)
        .await
        .unwrap_err();
    assert!(error.client_message().contains("timed out"), "{error}");
    assert!(
        server
            .visits
            .lock()
            .await
            .iter()
            .all(|visit| visit.uri != "/end")
    );
    assert_eq!(
        client
            .get(&server.address, HeaderMap::new(), 5.0)
            .await
            .unwrap()
            .body
            .as_ref(),
        b"done"
    );
}
