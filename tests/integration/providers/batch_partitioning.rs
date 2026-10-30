use crate::assertions::JsonProbe as _;
use crate::listener::LocalServer;
use alloc::sync::Arc;
use axum::{Json, Router, extract::State, http::HeaderMap, routing::post};
use rmcp::serde_json::{Value, json};
use tokio::sync::Mutex;
use web_rs::{net::secure_client, page::tinyfish::TinyFishFetchClient};
type Captures = Arc<Mutex<Vec<(HeaderMap, Value)>>>;
async fn reversed_batch(
    State(captured): State<Captures>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Json<Value> {
    let urls = body.list("/urls");
    let results = urls . iter () . rev () . map (| url | json ! ({ "url" : url , "text" : format ! ("page:{}" , url . as_str () . unwrap ()) })) . collect :: < Vec < _ > > () ;
    captured.lock().await.push((headers, body));
    Json(json ! ({ "results" : results , "errors" : [] }))
}
#[tokio::test]
async fn tinyfish_partitions_batches_and_restores_content_to_request_order() {
    let captures = Captures::default();
    let server = LocalServer::start(
        Router::new()
            .route("/api", post(reversed_batch))
            .with_state(Arc::clone(&captures)),
    )
    .await;
    let settings = server.settings();
    let client = TinyFishFetchClient::new(
        Arc::new(settings.clone()),
        secure_client(&settings.http, &settings.ssrf).unwrap(),
    );
    let limit = settings.tinyfish.max_urls_per_request;
    for count in [limit, limit + 1, limit * 2 + 1] {
        captures.lock().await.clear();
        let urls = (0..count)
            .map(|index| format!("https://example.org/{index}"))
            .collect::<Vec<_>>();
        let pages = client
            .read_markdown_many(&urls, "tinyfish-key")
            .await
            .unwrap();
        assert_eq!(pages.len(), urls.len());
        for (url, page) in urls.iter().zip(pages) {
            assert_eq!(page.unwrap(), format!("page:{url}"));
        }
        let requests = captures.lock().await;
        let mut sizes = requests
            .iter()
            .map(|capture| capture.1.list("/urls").len())
            .collect::<Vec<_>>();
        sizes.sort_unstable();
        let mut expected = urls.chunks(limit).map(<[String]>::len).collect::<Vec<_>>();
        expected.sort_unstable();
        assert_eq!(sizes, expected);
        for capture in requests.iter() {
            let headers = &capture.0;
            let body = &capture.1;
            assert_eq!(headers.get("x-api-key").unwrap(), "tinyfish-key");
            assert_eq!(
                body.get("format").unwrap(),
                &json!(settings.tinyfish.format)
            );
            assert_eq!(
                body.get("per_url_timeout_ms").unwrap(),
                &json!(settings.tinyfish.per_url_timeout_ms)
            );
        }
    }
}
