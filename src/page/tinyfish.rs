use crate::{
    Result,
    config::AppConfig,
    error::{AppError, http_service_error},
    net::SecureHttpClient,
};
use reqwest::header::{CONTENT_TYPE, HeaderMap, HeaderValue};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
#[cfg(test)]
mod tests;
#[derive(Clone)]
pub struct TinyFishFetchClient {
    config: AppConfig,
    http: SecureHttpClient,
}
#[derive(Serialize)]
struct TinyFishPayload<'request> {
    urls: Vec<&'request str>,
    format: &'request str,
    per_url_timeout_ms: u64,
}
#[derive(Deserialize)]
struct TinyFishResponse {
    results: Vec<TinyFishResult>,
    errors: Vec<TinyFishError>,
}
#[derive(Deserialize)]
struct TinyFishResult {
    url: String,
    text: String,
}
#[derive(Deserialize)]
struct TinyFishError {
    url: String,
    error: String,
    status: Option<u16>,
}
impl TinyFishFetchClient {
    #[inline]
    #[must_use]
    pub const fn new(config: AppConfig, http: SecureHttpClient) -> Self {
        Self { config, http }
    }
    #[expect(
        clippy::missing_inline_in_public_items,
        reason = "TinyFish reads perform async HTTP I/O and are not inline candidates."
    )]
    pub async fn read_markdown(&self, url: &str, api_key: &str) -> Result<String> {
        let urls = vec![url.to_owned()];
        self.read_markdown_many(&urls, api_key)
            .await?
            .into_iter()
            .next()
            .ok_or_else(|| AppError::internal("TinyFish batch response was empty"))?
    }
    #[expect(
        clippy::missing_inline_in_public_items,
        reason = "TinyFish batch reads perform async HTTP I/O and are not inline candidates."
    )]
    pub async fn read_markdown_many(
        &self,
        urls: &[String],
        api_key: &str,
    ) -> Result<Vec<Result<String>>> {
        let headers = headers(api_key)?;
        let payload = TinyFishPayload {
            urls: urls.iter().map(String::as_str).collect(),
            format: &self.config.tinyfish.format,
            per_url_timeout_ms: self.config.tinyfish.per_url_timeout_ms,
        };
        let body = sonic_rs::to_vec(&payload).map_err(|error| {
            AppError::internal(format!("failed to encode TinyFish request: {error}"))
        })?;
        let response = self
            .http
            .post(
                &self.config.tinyfish.endpoint,
                headers,
                body,
                self.config.http.timeout_seconds,
            )
            .await?;
        if response.status.as_u16() >= 400 {
            return Err(http_service_error("TinyFish", response.status.as_u16()));
        }
        extract_markdowns(urls, &response.body)
    }
}
fn headers(api_key: &str) -> Result<HeaderMap> {
    let mut headers = HeaderMap::new();
    headers.insert(
        "X-API-Key",
        HeaderValue::from_str(api_key).map_err(header_error)?,
    );
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    Ok(headers)
}
fn extract_markdowns(urls: &[String], body: &[u8]) -> Result<Vec<Result<String>>> {
    let payload = sonic_rs::from_slice::<TinyFishResponse>(body).map_err(|error| {
        AppError::client(format!(
            "TinyFish returned an unsupported response: {error}"
        ))
    })?;
    let results = payload
        .results
        .into_iter()
        .map(|result| (result.url, result.text))
        .collect::<HashMap<_, _>>();
    let errors = payload
        .errors
        .into_iter()
        .map(|error| (error.url.clone(), error))
        .collect::<HashMap<_, _>>();
    let mut markdowns = Vec::with_capacity(urls.len());
    for url in urls {
        if let Some(text) = results.get(url) {
            markdowns.push(Ok(text.clone()));
        } else if let Some(error) = errors.get(url) {
            markdowns.push(Err(tinyfish_fetch_error(error)));
        } else {
            markdowns.push(Err(AppError::client(format!(
                "TinyFish returned no content for the requested URL: {url}."
            ))));
        }
    }
    Ok(markdowns)
}
fn tinyfish_fetch_error(error: &TinyFishError) -> AppError {
    let status = error
        .status
        .map_or_else(String::new, |value| format!(" with HTTP {value}"));
    AppError::client(format!(
        "TinyFish could not fetch {}: {}{}.",
        error.url, error.error, status
    ))
}
#[expect(
    clippy::needless_pass_by_value,
    reason = "map_err passes InvalidHeaderValue by value and the formatter consumes only its Display output."
)]
fn header_error(error: reqwest::header::InvalidHeaderValue) -> AppError {
    AppError::internal(format!("invalid TinyFish header value: {error}"))
}
