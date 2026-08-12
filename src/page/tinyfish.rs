use crate::{Result, config::AppConfig, error::AppError, net::SecureHttpClient};
use futures::future::join_all;
use reqwest::header::{HeaderMap, HeaderValue};
use serde::Serialize;
mod response;
#[cfg(test)]
mod tests;
use response::{extract_markdowns, service_error};
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
        let batches = urls
            .chunks(self.config.tinyfish.max_urls_per_request)
            .map(|batch| self.read_batch(batch, api_key));
        let grouped = join_all(batches).await;
        Ok(grouped.into_iter().flatten().collect())
    }
    async fn read_batch(&self, urls: &[String], api_key: &str) -> Vec<Result<String>> {
        match self.request_batch(urls, api_key).await {
            Ok(markdowns) => markdowns,
            Err(error) => core::iter::repeat_with(|| Err(error.clone()))
                .take(urls.len())
                .collect(),
        }
    }
    async fn request_batch(&self, urls: &[String], api_key: &str) -> Result<Vec<Result<String>>> {
        let headers = headers(api_key)?;
        let payload = TinyFishPayload {
            urls: urls.iter().map(String::as_str).collect(),
            format: &self.config.tinyfish.format,
            per_url_timeout_ms: self.config.tinyfish.per_url_timeout_ms,
        };
        let response = self
            .http
            .post_json(
                &self.config.tinyfish.endpoint,
                headers,
                &payload,
                self.config.http.timeout_seconds,
            )
            .await?;
        if response.status.as_u16() >= 400 {
            return Err(service_error(response.status.as_u16(), &response.body));
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
    Ok(headers)
}
#[expect(
    clippy::needless_pass_by_value,
    reason = "map_err passes InvalidHeaderValue by value and the formatter consumes only its Display output."
)]
fn header_error(error: reqwest::header::InvalidHeaderValue) -> AppError {
    AppError::internal(format!("invalid TinyFish header value: {error}"))
}
