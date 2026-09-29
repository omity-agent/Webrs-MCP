use crate::{
    Result,
    config::HttpConfig,
    error::AppError,
    net::{SsrfGuard, body, transport},
};
use alloc::sync::Arc;
use bytes::Bytes;
use core::time::Duration;
use reqwest::{
    StatusCode, Url,
    header::{CONTENT_TYPE, HeaderMap, HeaderValue},
};
use serde::Serialize;
use tokio::sync::Semaphore;
#[derive(Clone)]
pub struct SecureHttpClient {
    direct: reqwest::Client,
    api: reqwest::Client,
    guard: SsrfGuard,
    requests: Arc<Semaphore>,
    max_response_bytes: usize,
    user_agent: HeaderValue,
}
#[derive(Clone, Debug)]
pub struct FetchResponse {
    pub status: StatusCode,
    pub headers: HeaderMap,
    pub body: Bytes,
}
impl SecureHttpClient {
    #[inline]
    pub fn new(config: &HttpConfig, guard: SsrfGuard) -> Result<Self> {
        if config.max_concurrent_requests == 0
            || config.max_concurrent_requests > Semaphore::MAX_PERMITS
        {
            return Err(AppError::config(
                "http.max_concurrent_requests is outside the supported range",
            ));
        }
        if config.max_response_bytes == 0 {
            return Err(AppError::config("http.max_response_bytes must be positive"));
        }
        let user_agent = HeaderValue::from_str(&config.user_agent)
            .map_err(|error| AppError::config(format!("http.user_agent: {error}")))?;
        let (direct, api) = transport::clients(&guard, &user_agent, config.max_redirects)?;
        Ok(Self {
            direct,
            api,
            guard,
            requests: Arc::new(Semaphore::new(config.max_concurrent_requests)),
            max_response_bytes: config.max_response_bytes,
            user_agent,
        })
    }
    #[inline]
    #[must_use]
    pub fn user_agent(&self) -> HeaderValue {
        self.user_agent.clone()
    }
    #[expect(
        clippy::missing_inline_in_public_items,
        reason = "URL validation performs DNS I/O within the configured time budget."
    )]
    pub async fn validate_url(&self, url: &Url, timeout_seconds: f64) -> Result<()> {
        tokio::time::timeout(duration(timeout_seconds)?, self.guard.validate_url(url))
            .await
            .map_err(|_elapsed| timeout_error())?
    }
    #[expect(
        clippy::missing_inline_in_public_items,
        reason = "HTTP GET performs async network I/O."
    )]
    pub async fn get(
        &self,
        url: &str,
        headers: HeaderMap,
        timeout_seconds: f64,
    ) -> Result<FetchResponse> {
        self.get_with_body_limit(url, headers, timeout_seconds, self.max_response_bytes)
            .await
    }
    #[expect(
        clippy::missing_inline_in_public_items,
        reason = "HTTP GET performs async network I/O."
    )]
    pub async fn get_with_body_limit(
        &self,
        url: &str,
        headers: HeaderMap,
        timeout_seconds: f64,
        max_bytes: usize,
    ) -> Result<FetchResponse> {
        self.execute(
            self.direct.get(self.checked_url(url)?).headers(headers),
            timeout_seconds,
            max_bytes,
        )
        .await
    }
    pub(crate) async fn post_json<Payload>(
        &self,
        url: &str,
        headers: HeaderMap,
        payload: &Payload,
        timeout_seconds: f64,
    ) -> Result<FetchResponse>
    where
        Payload: Serialize + Sync,
    {
        let encoded = sonic_rs::to_vec(payload).map_err(|error| {
            AppError::internal(format!("failed to encode HTTP JSON request: {error}"))
        })?;
        let request = self
            .api
            .post(self.checked_url(url)?)
            .headers(headers)
            .header(CONTENT_TYPE, "application/json")
            .body(encoded);
        self.execute(request, timeout_seconds, self.max_response_bytes)
            .await
    }
    async fn execute(
        &self,
        builder: reqwest::RequestBuilder,
        timeout_seconds: f64,
        max_bytes: usize,
    ) -> Result<FetchResponse> {
        let (client, request) = builder.build_split();
        let prepared = request?;
        let pending = async {
            let _permit = self.requests.acquire().await.map_err(|error| {
                AppError::internal(format!("HTTP request limiter was closed: {error}"))
            })?;
            let response = client.execute(prepared).await?;
            let status = response.status();
            let headers = response.headers().clone();
            let body = body::collect(response, max_bytes.min(self.max_response_bytes)).await?;
            Ok(FetchResponse {
                status,
                headers,
                body,
            })
        };
        tokio::time::timeout(duration(timeout_seconds)?, pending)
            .await
            .map_err(|_elapsed| timeout_error())?
    }
    fn checked_url(&self, url: &str) -> Result<Url> {
        let parsed =
            Url::parse(url).map_err(|error| AppError::client(format!("Invalid URL: {error}")))?;
        self.guard.check_url(&parsed)?;
        Ok(parsed)
    }
}
fn duration(seconds: f64) -> Result<Duration> {
    let parsed = Duration::try_from_secs_f64(seconds)
        .map_err(|error| AppError::config(format!("HTTP timeout is invalid: {error}")))?;
    if parsed.is_zero() {
        return Err(AppError::config("HTTP timeout must be positive"));
    }
    Ok(parsed)
}
fn timeout_error() -> AppError {
    AppError::Upstream("upstream request timed out. Retry later.".to_owned())
}
