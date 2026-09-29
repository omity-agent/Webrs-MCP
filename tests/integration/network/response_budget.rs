use super::fixture::Listener;
use axum::{Router, body::Body, http::HeaderMap, routing::get};
use bytes::Bytes;
use core::convert::Infallible;
use futures::stream;
use web_rs::{config, net::secure_client};
#[tokio::test]
async fn body_limit_covers_declared_and_streamed_lengths() {
    let server = Listener::start(
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
    let mut settings = config::load_embedded().unwrap();
    settings.ssrf.block_private_networks = false;
    let client = secure_client(&settings.http, &settings.ssrf).unwrap();
    for path in ["/declared", "/streamed"] {
        let url = format!("{}{path}", server.address);
        let response = client
            .get_with_body_limit(&url, HeaderMap::new(), 5.0, 6)
            .await
            .unwrap();
        assert_eq!(response.body.as_ref(), b"abcdef");
        let error = client
            .get_with_body_limit(&url, HeaderMap::new(), 5.0, 5)
            .await
            .unwrap_err();
        assert!(error.client_message().contains("5 byte limit"));
    }
}
