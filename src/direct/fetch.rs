use crate::{
    Result,
    config::{DirectFetchConfig, HttpConfig},
    direct::target::{DirectFetchTarget, ResponseFormat},
    error::AppError,
    net::{FetchResponse, SecureHttpClient},
};
use futures_util::future::{BoxFuture, Shared, join};
use probe::extract_direct_content;
use reqwest::header::{ACCEPT, HeaderMap, HeaderValue, USER_AGENT};
pub type SharedProbeFetch = Shared<BoxFuture<'static, Result<FetchResponse>>>;
#[expect(
    clippy::missing_inline_in_public_items,
    reason = "The public direct fetch entrypoint performs HTTP I/O and keeps protocol terminology."
)]
pub async fn fetch_direct_text(
    client: &SecureHttpClient,
    target: &DirectFetchTarget,
    direct_config: &DirectFetchConfig,
    http_config: &HttpConfig,
) -> Result<String> {
    let headers = request_headers(client, target)?;
    let (response, probe_response) =
        fetch_target_responses(client, target, headers, direct_config, http_config).await?;
    extract_direct_content(target, &response, probe_response, direct_config)
}
#[expect(
    clippy::missing_inline_in_public_items,
    reason = "The public direct fetch entrypoint performs HTTP I/O and keeps protocol terminology."
)]
pub async fn fetch_direct_text_with_probe(
    client: &SecureHttpClient,
    target: &DirectFetchTarget,
    direct_config: &DirectFetchConfig,
    http_config: &HttpConfig,
    probe_fetch: SharedProbeFetch,
) -> Result<String> {
    let headers = request_headers(client, target)?;
    let main_fetch = fetch_response(
        client,
        &target.request_url,
        headers,
        direct_config,
        http_config,
    );
    let (response_result, probe_result) = join(main_fetch, probe_fetch).await;
    let response = response_result?;
    extract_direct_content(target, &response, Some(probe_result), direct_config)
}
#[inline]
pub fn shared_probe_fetch(
    client: SecureHttpClient,
    probe_url: String,
    target: &DirectFetchTarget,
    direct_config: &DirectFetchConfig,
    http_config: &HttpConfig,
) -> Result<SharedProbeFetch> {
    probe::shared_probe_fetch(client, probe_url, target, direct_config, http_config)
}
async fn fetch_target_responses(
    client: &SecureHttpClient,
    target: &DirectFetchTarget,
    headers: HeaderMap,
    direct_config: &DirectFetchConfig,
    http_config: &HttpConfig,
) -> Result<(FetchResponse, Option<Result<FetchResponse>>)> {
    let Some(probe_url) = target.similarity_probe_url.as_deref() else {
        return Ok((
            fetch_response(
                client,
                &target.request_url,
                headers,
                direct_config,
                http_config,
            )
            .await?,
            None,
        ));
    };
    let main_fetch = fetch_response(
        client,
        &target.request_url,
        headers.clone(),
        direct_config,
        http_config,
    );
    let probe_fetch = fetch_response(client, probe_url, headers, direct_config, http_config);
    let (main_result, probe_result) = join(main_fetch, probe_fetch).await;
    Ok((main_result?, Some(probe_result)))
}
async fn fetch_response(
    client: &SecureHttpClient,
    url: &str,
    headers: HeaderMap,
    direct_config: &DirectFetchConfig,
    http_config: &HttpConfig,
) -> Result<FetchResponse> {
    client
        .get_with_body_limit(
            url,
            headers,
            http_config.direct_fetch_timeout_seconds,
            direct_config.max_bytes,
        )
        .await
}
fn request_headers(client: &SecureHttpClient, target: &DirectFetchTarget) -> Result<HeaderMap> {
    let mut headers = HeaderMap::new();
    headers.insert(
        ACCEPT,
        HeaderValue::from_str(&accept_header(target)).map_err(header_error)?,
    );
    headers.insert(USER_AGENT, client.user_agent());
    Ok(headers)
}
fn accept_header(target: &DirectFetchTarget) -> String {
    if let Some(value) = target.accept_header.clone() {
        return value;
    }
    if matches!(
        target.response_format,
        ResponseFormat::MediaWikiApi
            | ResponseFormat::PackageRegistryJson
            | ResponseFormat::StackOverflowQuestionJson
    ) {
        return "application/json".to_owned();
    }
    "text/plain,*/*".to_owned()
}
#[expect(
    clippy::needless_pass_by_value,
    reason = "map_err passes InvalidHeaderValue by value and the formatter consumes only its Display output."
)]
fn header_error(error: reqwest::header::InvalidHeaderValue) -> AppError {
    AppError::internal(format!("invalid configured HTTP header: {error}"))
}
mod probe;
#[cfg(test)]
mod tests;
