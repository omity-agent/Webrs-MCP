use crate::listener::LocalServer;
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
pub(super) async fn receive(
    format: ResponseFormat,
    status: StatusCode,
    content_type: Option<&'static str>,
    body: &'static [u8],
) -> web_rs::Result<String> {
    let server = LocalServer::start(Router::new().route(
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
    let settings = server.settings();
    let url = format!("{}/content", server.address);
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
async fn direct_content_requires_complete_success_and_a_valid_non_html_media_type() {
    for (format, content_type) in [
        (ResponseFormat::Markdown, None),
        (ResponseFormat::Markdown, Some("not-a-mime")),
        (ResponseFormat::Markdown, Some("text/html")),
        (ResponseFormat::Markdown, Some("application/xhtml+xml")),
        (ResponseFormat::PackageRegistryJson, Some("text/html")),
    ] {
        let error = receive(format, StatusCode::OK, content_type, b"{}")
            .await
            .unwrap_err();
        assert!(error.client_message().contains("Content-Type"), "{error}");
    }
    for status in [
        StatusCode::NO_CONTENT,
        StatusCode::PARTIAL_CONTENT,
        StatusCode::NOT_MODIFIED,
        StatusCode::NOT_FOUND,
        StatusCode::INTERNAL_SERVER_ERROR,
    ] {
        let error = receive(
            ResponseFormat::Markdown,
            status,
            Some("text/markdown"),
            b"# Page",
        )
        .await
        .unwrap_err();
        assert!(
            error
                .client_message()
                .contains(&status.as_u16().to_string()),
            "{error}"
        );
    }
    assert_eq!(
        receive(
            ResponseFormat::Markdown,
            StatusCode::OK,
            Some("Text/Markdown; charset=utf-8"),
            b"# Page"
        )
        .await
        .unwrap(),
        "# Page"
    );
}
#[tokio::test]
async fn markdown_rejects_binary_and_html_documents_but_preserves_embedded_markup_and_raw_sources()
{
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
    let markdown = b"# Page\r\n\r\n<div>Embedded HTML</div>";
    assert_eq!(
        receive(
            ResponseFormat::Markdown,
            StatusCode::OK,
            Some("text/markdown"),
            markdown
        )
        .await
        .unwrap(),
        "# Page\r\n\r\n<div>Embedded HTML</div>"
    );
    assert_eq!(
        receive(
            ResponseFormat::Text,
            StatusCode::OK,
            Some("text/plain"),
            b"<!DOCTYPE html><html>Source</html>"
        )
        .await
        .unwrap(),
        "<!DOCTYPE html><html>Source</html>"
    );
}
