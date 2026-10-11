use crate::{listener::LocalServer, upstream::Upstream};
use alloc::sync::Arc;
use axum::{
    Json, Router,
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode},
    routing::post,
};
use rmcp::serde_json::{Value, json};
use tokio::sync::Mutex;
use web_rs::{net::secure_client, page::tinyfish::TinyFishFetchClient};
type Captures = Arc<Mutex<Vec<(HeaderMap, Value)>>>;
async fn reversed_batch(
    State(captured): State<Captures>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Json<Value> {
    let urls = body.get("urls").unwrap().as_array().unwrap();
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
            .map(|capture| capture.1.get("urls").unwrap().as_array().unwrap().len())
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
#[tokio::test]
async fn tinyfish_retains_mixed_success_failure_and_missing_entries_in_place() {
    let upstream = Upstream :: json (& json ! ({ "results" : [{ "url" : "https://example.org/b" , "text" : "# B" } , { "url" : "https://example.org/a" , "text" : "# A" }] , "errors" : [{ "url" : "https://example.org/blocked" , "error" : "bot_blocked" , "status" : 403_u16 }] })) . await ;
    let settings = upstream.server.settings();
    let client = TinyFishFetchClient::new(
        Arc::new(settings.clone()),
        secure_client(&settings.http, &settings.ssrf).unwrap(),
    );
    let urls = ["a", "blocked", "b", "missing"].map(|path| format!("https://example.org/{path}"));
    let results = client.read_markdown_many(&urls, "key").await.unwrap();
    assert_eq!(results.len(), 4);
    let [first, blocked_result, second, missing_result]: [_; 4] = results.try_into().unwrap();
    assert_eq!(first.unwrap(), "# A");
    assert_eq!(second.unwrap(), "# B");
    let blocked = blocked_result.unwrap_err().client_message();
    assert!(
        blocked.contains(&urls[1]) && blocked.contains("bot_blocked") && blocked.contains("403"),
        "{blocked}"
    );
    let missing = missing_result.unwrap_err().client_message();
    assert!(
        missing.contains(&urls[3]) && missing.contains("no content"),
        "{missing}"
    );
    assert_eq!(upstream.requests.lock().await.len(), 1);
    let rejected = Upstream :: start (StatusCode :: BAD_REQUEST , "application/json" , Bytes :: from_static (br#"{"error":{"code":"INVALID_INPUT","message":"Request rejected."},"request_id":"request-123"}"#) ,) . await ;
    let rejected_settings = rejected.server.settings();
    let rejected_client = TinyFishFetchClient::new(
        Arc::new(rejected_settings.clone()),
        secure_client(&rejected_settings.http, &rejected_settings.ssrf).unwrap(),
    );
    let failures = rejected_client
        .read_markdown_many(&urls, "key")
        .await
        .unwrap();
    assert_eq!(failures.len(), urls.len());
    for failure in failures {
        let message = failure.unwrap_err().client_message();
        for detail in [
            "HTTP 400",
            "INVALID_INPUT",
            "Request rejected",
            "request-123",
        ] {
            assert!(message.contains(detail), "{message}");
        }
    }
    assert_eq!(rejected.requests.lock().await.len(), 1);
}
