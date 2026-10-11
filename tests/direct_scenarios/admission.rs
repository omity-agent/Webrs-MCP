use super::environment::Website;
use axum::{
    Router,
    body::Body,
    http::{HeaderMap, StatusCode, header::CONTENT_TYPE},
    response::Response,
    routing::get,
};
use web_rs::{
    direct::{DirectFetchTarget, ResponseFormat, content::extract_content},
    net::secure_client,
};
#[tokio::test]
async fn learn_html_response_is_not_accepted_as_markdown() {
    let website = Website::start(Router::new().route(
        "/en-us/windows/article",
        get(|| async {
            (
                [("content-type", "text/html")],
                "<!DOCTYPE html><html>Page</html>",
            )
        }),
    ))
    .await;
    let mut settings = website.settings();
    settings.direct_fetch.microsoft_learn_hosts = vec!["127.0.0.1".to_owned()];
    let result = website
        .fetcher(&settings)
        .fetch(&format!("{}/en-us/windows/article", website.address), None)
        .await;
    assert!(result.is_err(), "Learn HTML was accepted as Markdown");
}
#[tokio::test]
async fn registry_json_with_html_content_type_is_rejected() {
    let website = Website::start(
        Router::new()
            .route(
                "/package/demo",
                get(|| async { ([("content-type", "text/html")], "<html>Package</html>") }),
            )
            .route(
                "/registry/demo",
                get(|| async { ([("content-type", "text/html")], r#"{"name":"demo"}"#) }),
            ),
    )
    .await;
    let mut settings = website.settings();
    settings.direct_fetch.npm_hosts = vec!["127.0.0.1".to_owned()];
    settings.direct_fetch.npm_registry_url_prefix = format!("{}/registry/", website.address);
    let result = website
        .fetcher(&settings)
        .fetch(&format!("{}/package/demo", website.address), None)
        .await;
    assert!(
        result.is_err(),
        "registry bypassed response type validation"
    );
}
async fn receive(
    format: ResponseFormat,
    status: StatusCode,
    content_type: Option<&'static str>,
    body: &'static [u8],
) -> web_rs::Result<String> {
    let website = Website::start(Router::new().route(
        "/content",
        get(move || async move {
            let mut builder = Response::builder().status(status);
            if let Some(value) = content_type {
                builder = builder.header(CONTENT_TYPE, value);
            }
            builder.body(Body::from(body)).unwrap()
        }),
    ))
    .await;
    let settings = website.settings();
    let url = format!("{}/content", website.address);
    let response = secure_client(&settings.http, &settings.ssrf)
        .unwrap()
        .get(&url, HeaderMap::new(), 5.0)
        .await?;
    let mut target = DirectFetchTarget::text(&url, &url);
    target.response_format = format;
    extract_content(
        &target,
        response.status.as_u16(),
        &response.headers,
        &response.body,
    )
}
#[tokio::test]
async fn every_direct_format_requires_a_valid_non_html_media_type() {
    for format in [
        ResponseFormat::Markdown,
        ResponseFormat::Text,
        ResponseFormat::MediaWikiApi,
        ResponseFormat::PackageRegistryJson,
        ResponseFormat::StackOverflowQuestionJson,
    ] {
        for content_type in [
            None,
            Some("not-a-mime"),
            Some("text/html"),
            Some("application/xhtml+xml"),
        ] {
            let result = receive(format, StatusCode::OK, content_type, b"{}").await;
            assert!(
                result
                    .unwrap_err()
                    .client_message()
                    .contains("Content-Type"),
                "{format:?} bypassed media type validation"
            );
        }
    }
}
#[tokio::test]
async fn incomplete_and_non_success_responses_are_rejected() {
    for status in [
        StatusCode::NO_CONTENT,
        StatusCode::PARTIAL_CONTENT,
        StatusCode::NOT_MODIFIED,
        StatusCode::NOT_FOUND,
        StatusCode::INTERNAL_SERVER_ERROR,
    ] {
        assert!(
            receive(
                ResponseFormat::Markdown,
                status,
                Some("text/markdown"),
                b"# Page"
            )
            .await
            .is_err(),
            "{status} was accepted as complete page content"
        );
    }
}
#[tokio::test]
async fn markdown_body_validation_preserves_embedded_html_but_rejects_html_documents() {
    for body in [
        b"<!DOCTYPE html><html>Page</html>".as_slice(),
        b"\xEF\xBB\xBF<!-- comment -->\n<HTML lang=\"en\">Page</HTML>".as_slice(),
        b"\xFF\xFEbroken text".as_slice(),
        b"\0binary content".as_slice(),
    ] {
        receive(
            ResponseFormat::Markdown,
            StatusCode::OK,
            Some("text/markdown"),
            body,
        )
        .await
        .unwrap_err();
    }
    let markdown = b"# Page\n\n<div>Embedded HTML</div>";
    assert_eq!(
        receive(
            ResponseFormat::Markdown,
            StatusCode::OK,
            Some("Text/Markdown; charset=utf-8"),
            markdown,
        )
        .await
        .unwrap(),
        core::str::from_utf8(markdown).unwrap()
    );
    assert_eq!(
        receive(
            ResponseFormat::Text,
            StatusCode::OK,
            Some("text/plain"),
            b"<!DOCTYPE html><html>Source code</html>",
        )
        .await
        .unwrap(),
        "<!DOCTYPE html><html>Source code</html>"
    );
}
#[tokio::test]
async fn valid_registry_json_keeps_its_structured_content() {
    let content = receive(
        ResponseFormat::PackageRegistryJson,
        StatusCode::OK,
        Some("application/vnd.registry+json; charset=utf-8"),
        br#"{"name":"demo","description":"Package"}"#,
    )
    .await
    .unwrap();
    assert!(content.contains("\"name\": \"demo\""));
}
