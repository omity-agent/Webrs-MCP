use crate::listener::LocalServer;
use axum::{Router, http::HeaderMap, routing::get};
use web_rs::{config, error::AppError, net::secure_client};
#[tokio::test]
async fn ssrf_blocks_local_targets_before_io_and_allows_explicit_local_policy() {
    let server = LocalServer::start(Router::new().route("/", get(|| async { "allowed" }))).await;
    let settings = config::load_embedded().unwrap();
    let client = secure_client(&settings.http, &settings.ssrf).unwrap();
    let local_hostname = server.address.replace("127.0.0.1", "localhost");
    for url in [&server.address, &local_hostname] {
        let error = client.get(url, HeaderMap::new(), 5.0).await.unwrap_err();
        assert!(matches!(error, AppError::Client(_)), "{error}");
        assert!(error.client_message().contains("blocked"), "{error}");
    }
    assert_eq!(server.uris().await.len(), 0);
    let local = server.settings();
    let permitted = secure_client(&local.http, &local.ssrf).unwrap();
    assert_eq!(
        permitted
            .get(&local_hostname, HeaderMap::new(), 5.0)
            .await
            .unwrap()
            .body
            .as_ref(),
        b"allowed"
    );
    assert_eq!(server.uris().await.len(), 1);
}
