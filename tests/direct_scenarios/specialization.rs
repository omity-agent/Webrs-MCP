use super::environment::Website;
use axum::{Router, extract::Query, response::Redirect, routing::get};
use std::collections::HashMap;
#[tokio::test]
async fn learn_markdown_rule_is_applied_after_shortlink_redirection() {
    let website = Website::start(
        Router::new()
            .route(
                "/short",
                get(|| async { Redirect::temporary("/en-us/windows/article") }),
            )
            .route(
                "/en-us/windows/article",
                get(|Query(query): Query<HashMap<String, String>>| async move {
                    if query
                        .get("accept")
                        .is_some_and(|value| value == "text/markdown")
                    {
                        ([("content-type", "text/markdown")], "# Learn document")
                    } else {
                        (
                            [("content-type", "text/html")],
                            "<html>Learn document</html>",
                        )
                    }
                }),
            ),
    )
    .await;
    let mut settings = website.settings();
    settings.direct_fetch.microsoft_learn_hosts = vec!["127.0.0.1".to_owned()];
    let page = website
        .fetcher(&settings)
        .fetch(&format!("{}/short", website.address), None)
        .await
        .unwrap();
    assert_eq!(page.markdown.as_ref(), "# Learn document");
    let visits = website.visits.lock().await;
    assert!(
        visits
            .iter()
            .any(|visit| visit.uri == "/en-us/windows/article?accept=text%2Fmarkdown")
    );
    assert!(visits.iter().all(|visit| !visit.uri.starts_with("/short?")));
    drop(visits);
}
#[tokio::test]
async fn learn_non_document_sections_do_not_get_a_markdown_query() {
    let website = Website::start(Router::new().route(
        concat!("/{", "locale", "}/{", "section", "}/page"),
        get(|| async {
            (
                [("content-type", "text/html")],
                "<html>Not documentation</html>",
            )
        }),
    ))
    .await;
    let mut settings = website.settings();
    settings.direct_fetch.microsoft_learn_hosts = vec!["127.0.0.1".to_owned()];
    for (locale, section) in [
        ("en-us", "answers"),
        ("zh-cn", "training"),
        ("en-us", "shows"),
        ("en-us", "events"),
        ("%65n-us", "answers"),
        ("en-us", "%61nswers"),
        ("fil-ph", "training"),
        ("sr-latn-rs", "shows"),
    ] {
        let result = website
            .fetcher(&settings)
            .fetch(
                &format!("{}/{locale}/{section}/page", website.address),
                None,
            )
            .await;
        assert!(result.is_err(), "{section} HTML was accepted as Markdown");
    }
    assert!(
        website
            .visits
            .lock()
            .await
            .iter()
            .all(|visit| !visit.uri.contains("accept="))
    );
}
