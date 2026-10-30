use crate::assertions::JsonProbe as _;
use crate::listener::LocalServer;
use alloc::sync::Arc;
use axum::{
    Json, Router,
    routing::{get, post},
};
use rmcp::serde_json::{Value, json};
use web_rs::{
    net::secure_client,
    page::{PageFetcher, fetcher::PageSource, reader::ReaderCredentials},
};
fn client(server: &LocalServer, learn: bool) -> PageFetcher {
    let mut settings = server.settings();
    if learn {
        settings.direct_fetch.microsoft_learn_hosts = vec!["127.0.0.1".to_owned()];
    }
    let http = secure_client(&settings.http, &settings.ssrf).unwrap();
    PageFetcher::new(Arc::new(settings), http)
}
#[tokio::test]
async fn learn_strips_only_complete_leading_metadata_and_preserves_body_bytes() {
    for (markdown, expected) in [
        (
            "---\ntitle: Learn\nms.topic: reference\n---\n\n# 标题\n\n```yaml\n---\nkey: value\n---\n```\n\n---\n",
            "# 标题\n\n```yaml\n---\nkey: value\n---\n```\n\n---\n",
        ),
        (
            "\u{feff}---\r\nlayout: Conceptual\r\n---\r\n\r\n# Title\r\n\r\nBody\r\n",
            "# Title\r\n\r\nBody\r\n",
        ),
        ("---\ntitle: Learn\n---", ""),
        ("# Title\n\n---\n\nBody", "# Title\n\n---\n\nBody"),
        ("---\ntitle: Incomplete\n", "---\ntitle: Incomplete\n"),
    ] {
        let server = LocalServer::start(Router::new().route(
            "/en-us/docs/page",
            get(move || async move { ([("content-type", "text/markdown")], markdown) }),
        ))
        .await;
        let url = format!("{}/en-us/docs/page", server.address);
        let page = client(&server, true).fetch(&url, None).await.unwrap();
        assert_eq!(page.markdown.as_ref(), expected);
        assert_eq!(page.url, url);
        assert_eq!(page.source, PageSource::Direct);
        let untouched = client(&server, false).fetch(&url, None).await.unwrap();
        assert_eq!(untouched.markdown.as_ref(), markdown);
    }
}
#[tokio::test]
async fn learn_metadata_is_removed_from_single_and_batch_remote_reader_pages() {
    const MARKDOWN: &str = "---\ntitle: Learn\n---\n\n# Reader document";
    let server = LocalServer::start(
        Router::new()
            .route(
                "/jina",
                post(|| async { Json(json!({"data": {"markdown": MARKDOWN}})) }),
            )
            .route(
                "/tinyfish",
                post(|Json(body): Json<Value>| async move {
                    let results = body
                        .list("/urls")
                        .iter()
                        .map(|url| json!({"url": url, "text": MARKDOWN}))
                        .collect::<Vec<_>>();
                    Json(json!({"results": results, "errors": []}))
                }),
            ),
    )
    .await;
    let mut settings = server.settings();
    settings.direct_fetch.microsoft_learn_hosts = vec!["127.0.0.1".to_owned()];
    settings.jina.endpoint = format!("{}/jina", server.address);
    settings.tinyfish.endpoint = format!("{}/tinyfish", server.address);
    let http = secure_client(&settings.http, &settings.ssrf).unwrap();
    let fetcher = PageFetcher::new(Arc::new(settings), http);
    let urls = ["first", "second"].map(|name| format!("{}/en-us/docs/{name}", server.address));
    for credentials in [
        ReaderCredentials::Jina("key".to_owned()),
        ReaderCredentials::TinyFish("key".to_owned()),
    ] {
        let single = fetcher
            .fetch(urls.first().unwrap(), Some(&credentials))
            .await
            .unwrap();
        assert_eq!(single.markdown.as_ref(), "# Reader document");
        let pages = fetcher.fetch_many(&urls, Some(&credentials)).await.unwrap();
        assert_eq!(pages.len(), urls.len());
        for (page, url) in pages.iter().zip(&urls) {
            assert_eq!(page.markdown.as_ref(), "# Reader document");
            assert_eq!(&page.url, url);
            assert_eq!(page.source, PageSource::Reader);
        }
    }
}
