use crate::{
    Result,
    config::{DirectFetchConfig, HttpConfig},
    direct::{
        content::extract_content,
        target::{DirectFetchTarget, ResponseFormat},
    },
    net::{FetchResponse, SecureHttpClient},
};
use alloc::sync::Arc;
use futures::future::{BoxFuture, Shared, join};
use reqwest::header::{ACCEPT, HeaderMap, HeaderValue, USER_AGENT};
pub(crate) type SharedProbeFetch = Shared<BoxFuture<'static, Result<Option<Arc<str>>>>>;
pub(crate) async fn fetch_direct_text(
    client: &SecureHttpClient,
    target: &DirectFetchTarget,
    direct_config: &DirectFetchConfig,
    http_config: &HttpConfig,
    probe_fetch: Option<SharedProbeFetch>,
) -> Result<String> {
    let headers = request_headers(client, target);
    let main_fetch = async {
        let response = fetch_response(
            client,
            &target.request_url,
            headers,
            direct_config,
            http_config,
        )
        .await?;
        extract_content(
            target,
            response.status.as_u16(),
            &response.headers,
            &response.body,
        )
    };
    let probe = async {
        match probe_fetch {
            Some(fetch) => fetch.await,
            None => Ok(None),
        }
    };
    let (content_result, probe_result) = join(main_fetch, probe).await;
    let content = content_result?;
    if let Some(probe_content) = probe_result? {
        probe::reject_if_similar(&content, &probe_content, direct_config)?;
    }
    Ok(content)
}
#[inline]
pub(crate) fn shared_probe_fetch(
    client: SecureHttpClient,
    probe_url: String,
    target: &DirectFetchTarget,
    direct_config: &DirectFetchConfig,
    http_config: &HttpConfig,
) -> SharedProbeFetch {
    probe::shared_probe_fetch(client, probe_url, target, direct_config, http_config)
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
fn request_headers(client: &SecureHttpClient, target: &DirectFetchTarget) -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(ACCEPT, HeaderValue::from_static(accept_header(target)));
    headers.insert(USER_AGENT, client.user_agent());
    headers
}
const fn accept_header(target: &DirectFetchTarget) -> &'static str {
    match target.response_format {
        ResponseFormat::Markdown => "text/markdown",
        ResponseFormat::Text => "text/plain,*/*",
        ResponseFormat::MediaWikiApi
        | ResponseFormat::PackageRegistryJson
        | ResponseFormat::StackOverflowQuestionJson => "application/json",
    }
}
mod probe;
