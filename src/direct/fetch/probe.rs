use crate::{
    Result,
    config::{DirectFetchConfig, HttpConfig},
    direct::{content::extract_probe_content, target::DirectFetchTarget},
    error::AppError,
    net::SecureHttpClient,
};
use alloc::sync::Arc;
use futures::future::FutureExt as _;
#[inline]
pub(super) fn shared_probe_fetch(
    client: SecureHttpClient,
    probe_url: String,
    target: &DirectFetchTarget,
    direct_config: &DirectFetchConfig,
    http_config: &HttpConfig,
) -> super::SharedProbeFetch {
    let headers = super::request_headers(&client, target);
    let probe_target = target.clone();
    let timeout_seconds = http_config.direct_fetch_timeout_seconds;
    let max_bytes = direct_config.max_bytes;
    async move {
        let response = client
            .get_with_body_limit(&probe_url, headers, timeout_seconds, max_bytes)
            .await?;
        let content = extract_probe_content(
            &probe_target,
            response.status.as_u16(),
            &response.headers,
            &response.body,
        )?;
        Ok(content.map(Arc::from))
    }
    .boxed()
    .shared()
}
pub(super) fn reject_if_similar(
    content: &str,
    probe_content: &str,
    direct_config: &DirectFetchConfig,
) -> Result<()> {
    let similarity = strsim::normalized_levenshtein(content, probe_content);
    if similarity >= direct_config.similarity_threshold {
        return Err(AppError::client(format!(
            "Direct Markdown content is too similar to a known-missing URL response ({similarity:.3})."
        )));
    }
    Ok(())
}
