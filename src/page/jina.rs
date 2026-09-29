use crate::{
    Result,
    config::{AppConfig, JinaViewportConfig},
    error::{AppError, http_service_error},
    net::SecureHttpClient,
};
use finesse::{Frame, Parser};
use reqwest::header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE, HeaderMap, HeaderValue};
use serde::Serialize;
use sonic_rs::{JsonContainerTrait as _, JsonValueTrait as _, Value};
#[cfg(test)]
mod tests;
#[derive(Clone)]
pub struct JinaReaderClient {
    config: AppConfig,
    http: SecureHttpClient,
}
#[derive(Serialize)]
struct JinaPayload<'config> {
    url: String,
    viewport: &'config JinaViewportConfig,
}
impl JinaReaderClient {
    #[inline]
    #[must_use]
    pub const fn new(config: AppConfig, http: SecureHttpClient) -> Self {
        Self { config, http }
    }
    #[expect(
        clippy::missing_inline_in_public_items,
        reason = "Jina reads perform async HTTP I/O and are not inline candidates."
    )]
    pub async fn read_markdown(&self, url: &str, api_key: &str) -> Result<String> {
        let headers = self.headers(api_key)?;
        let payload = JinaPayload {
            url: rewrite_arxiv_pdf_url(url, &self.config),
            viewport: &self.config.jina.viewport,
        };
        let response = self
            .http
            .post_json(
                &self.config.jina.endpoint,
                headers,
                &payload,
                self.config.http.timeout_seconds,
            )
            .await?;
        if response.status.as_u16() >= 400 {
            return Err(http_service_error("Jina", response.status.as_u16()));
        }
        extract_content(&response.headers, &response.body)
    }
    fn headers(&self, api_key: &str) -> Result<HeaderMap> {
        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {api_key}")).map_err(header_error)?,
        );
        headers.insert(
            ACCEPT,
            HeaderValue::from_str(&self.config.jina.accept).map_err(header_error)?,
        );
        insert_header(&mut headers, "X-Engine", &self.config.jina.engine)?;
        insert_header(&mut headers, "X-Locale", &self.config.jina.locale)?;
        insert_header(
            &mut headers,
            "X-No-Cache",
            header_bool(self.config.jina.no_cache),
        )?;
        insert_header(
            &mut headers,
            "X-Respond-With",
            &self.config.jina.respond_with,
        )?;
        insert_header(
            &mut headers,
            "X-Retain-Images",
            &self.config.jina.retain_images,
        )?;
        insert_header(
            &mut headers,
            "X-Return-Format",
            &self.config.jina.return_format,
        )?;
        insert_header(
            &mut headers,
            "X-With-Shadow-Dom",
            header_bool(self.config.jina.with_shadow_dom),
        )?;
        Ok(headers)
    }
}
fn extract_content(headers: &HeaderMap, body: &[u8]) -> Result<String> {
    let is_event_stream = headers
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<mime::Mime>().ok())
        .is_some_and(|content_type| {
            content_type.essence_str() == mime::TEXT_EVENT_STREAM.essence_str()
        });
    if is_event_stream {
        return extract_event_stream_content(body);
    }
    sonic_rs::from_slice::<Value>(body).map_or_else(
        |_error| Ok(String::from_utf8_lossy(body).into_owned()),
        |payload| {
            extract_payload_content(&payload).ok_or_else(|| {
                AppError::client(
                    "Jina returned an unsupported response. Retry later or try another URL.",
                )
            })
        },
    )
}
fn extract_payload_content(payload: &Value) -> Option<String> {
    if let Some(object) = payload.as_object() {
        if let Some(data) = object.get(&"data")
            && let Some(content) = extract_payload_content(data)
        {
            return Some(content);
        }
        for key in ["content", "markdown", "text"] {
            if let Some(text) = object.get(&key).and_then(|value| value.as_str()) {
                return Some(text.to_owned());
            }
        }
    }
    payload.as_str().map(str::to_owned)
}
fn extract_event_stream_content(body: &[u8]) -> Result<String> {
    let mut parser = Parser::new();
    parser
        .feed(body)
        .map_err(|error| AppError::client(format!("Jina SSE parsing failed: {error}")))?;
    parser
        .end()
        .map_err(|error| AppError::client(format!("Jina SSE parsing failed: {error}")))?;
    let mut latest_content = None;
    while let Some(frame) = parser.next_frame() {
        if let Frame::Message(message) = frame {
            latest_content = event_stream_content(latest_content, &message.data);
        }
    }
    latest_content.ok_or_else(|| AppError::client("Jina SSE response contains no page content."))
}
fn event_stream_content(latest: Option<String>, data: &str) -> Option<String> {
    if data == "[DONE]" {
        return latest;
    }
    let Ok(payload) = sonic_rs::from_str::<Value>(data) else {
        return Some(data.to_owned());
    };
    extract_payload_content(&payload).or(latest)
}
fn insert_header(headers: &mut HeaderMap, name: &'static str, value: &str) -> Result<()> {
    headers.insert(name, HeaderValue::from_str(value).map_err(header_error)?);
    Ok(())
}
const fn header_bool(value: bool) -> &'static str {
    if value { "true" } else { "false" }
}
fn rewrite_arxiv_pdf_url(url: &str, config: &AppConfig) -> String {
    url.strip_prefix(&config.jina.arxiv_pdf_url_prefix)
        .map_or_else(
            || url.to_owned(),
            |suffix| format!("{}{suffix}", config.jina.arxiv_html_url_prefix),
        )
}
#[expect(
    clippy::needless_pass_by_value,
    reason = "map_err passes InvalidHeaderValue by value and the formatter consumes only its Display output."
)]
fn header_error(error: reqwest::header::InvalidHeaderValue) -> AppError {
    AppError::internal(format!("invalid Jina header value: {error}"))
}
