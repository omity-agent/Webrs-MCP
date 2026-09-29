use crate::{
    Result,
    error::AppError,
    net::{SsrfGuard, body, resolver::GuardedResolver},
};
use alloc::sync::Arc;
use bytes::Bytes;
use core::num::NonZeroUsize;
use core::time::Duration;
use redirect::redirect_target;
use reqwest::{
    Method, StatusCode, Url,
    header::{CONTENT_TYPE, HeaderMap, HeaderValue, LOCATION},
    redirect::Policy,
};
use serde_core::Serialize;
use tokio::sync::Semaphore;
mod redirect;
#[derive(Clone)]
pub struct SecureHttpClient {
    client: reqwest::Client,
    guard: SsrfGuard,
    max_redirects: usize,
    requests: Arc<Semaphore>,
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
    pub fn new(
        max_redirects: usize,
        max_concurrent_requests: usize,
        user_agent: &str,
        guard: SsrfGuard,
    ) -> Result<Self> {
        let request_limit = NonZeroUsize::new(max_concurrent_requests)
            .ok_or_else(|| AppError::config("http.max_concurrent_requests must be positive"))?;
        let user_agent_header = HeaderValue::from_str(user_agent)
            .map_err(|error| AppError::config(format!("http.user_agent: {error}")))?;
        let client = reqwest::Client::builder()
            .dns_resolver(Arc::new(GuardedResolver::new(guard.clone())))
            .redirect(Policy::none())
            .user_agent(user_agent_header.clone())
            .build()
            .map_err(|error| {
                AppError::internal(format!("failed to build guarded HTTP client: {error}"))
            })?;
        Ok(Self {
            client,
            guard,
            max_redirects,
            requests: Arc::new(Semaphore::new(request_limit.get())),
            user_agent: user_agent_header,
        })
    }
    #[inline]
    #[must_use]
    pub fn user_agent(&self) -> HeaderValue {
        self.user_agent.clone()
    }
    #[expect(
        clippy::missing_inline_in_public_items,
        reason = "HTTP GET performs async network I/O and is not an inline candidate."
    )]
    pub async fn get(
        &self,
        url: &str,
        headers: HeaderMap,
        timeout_seconds: f64,
    ) -> Result<FetchResponse> {
        let parsed =
            Url::parse(url).map_err(|error| AppError::client(format!("Invalid URL: {error}")))?;
        self.request_with_redirects(Method::GET, parsed, headers, None, timeout_seconds, None)
            .await
    }
    #[expect(
        clippy::missing_inline_in_public_items,
        reason = "HTTP GET performs async network I/O and is not an inline candidate."
    )]
    pub async fn get_with_body_limit(
        &self,
        url: &str,
        headers: HeaderMap,
        timeout_seconds: f64,
        max_bytes: usize,
    ) -> Result<FetchResponse> {
        let parsed =
            Url::parse(url).map_err(|error| AppError::client(format!("Invalid URL: {error}")))?;
        self.request_with_redirects(
            Method::GET,
            parsed,
            headers,
            None,
            timeout_seconds,
            Some(max_bytes),
        )
        .await
    }
    #[expect(
        clippy::missing_inline_in_public_items,
        reason = "HTTP POST performs async network I/O and is not an inline candidate."
    )]
    pub async fn post_json<Payload>(
        &self,
        url: &str,
        mut headers: HeaderMap,
        payload: &Payload,
        timeout_seconds: f64,
    ) -> Result<FetchResponse>
    where
        Payload: Serialize + Sync,
    {
        let parsed =
            Url::parse(url).map_err(|error| AppError::client(format!("Invalid URL: {error}")))?;
        let body = sonic_rs::to_vec(payload).map_err(|error| {
            AppError::internal(format!("failed to encode HTTP JSON request: {error}"))
        })?;
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        let _permit = self.requests.acquire().await.map_err(|error| {
            AppError::internal(format!(
                "HTTP request concurrency limiter was closed: {error}"
            ))
        })?;
        self.guard.validate_url(&parsed).await?;
        let response = self
            .client
            .post(parsed)
            .headers(headers)
            .body(body)
            .timeout(duration(timeout_seconds)?)
            .send()
            .await?;
        collect_response(response, None).await
    }
    async fn request_with_redirects(
        &self,
        method: Method,
        mut url: Url,
        headers: HeaderMap,
        body: Option<Vec<u8>>,
        timeout_seconds: f64,
        body_limit: Option<usize>,
    ) -> Result<FetchResponse> {
        let _permit = self.requests.acquire().await.map_err(|error| {
            AppError::internal(format!(
                "HTTP request concurrency limiter was closed: {error}"
            ))
        })?;
        for redirect_index in 0..=self.max_redirects {
            self.guard.validate_url(&url).await?;
            let response = self
                .client
                .request(method.clone(), url.clone())
                .headers(headers.clone())
                .body(body.clone().unwrap_or_default())
                .timeout(duration(timeout_seconds)?)
                .send()
                .await?;
            if !response.status().is_redirection() {
                return collect_response(response, body_limit).await;
            }
            let Some(next) = redirect_target(response.headers().get(LOCATION), &url)? else {
                return collect_response(response, body_limit).await;
            };
            if redirect_index == self.max_redirects {
                return Err(AppError::client("Too many redirects while fetching URL."));
            }
            url = next;
        }
        Err(AppError::internal("redirect loop exited unexpectedly"))
    }
}
async fn collect_response(
    response: reqwest::Response,
    body_limit: Option<usize>,
) -> Result<FetchResponse> {
    let status = response.status();
    let headers = response.headers().clone();
    let body = body::collect(response, body_limit).await?;
    Ok(FetchResponse {
        status,
        headers,
        body,
    })
}
fn duration(seconds: f64) -> Result<Duration> {
    if seconds.is_finite() && seconds > 0.0_f64 {
        return Ok(Duration::from_secs_f64(seconds));
    }
    Err(AppError::config("HTTP timeout must be positive"))
}
