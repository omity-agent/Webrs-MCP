use crate::{
    Result,
    config::{DirectFetchConfig, HttpConfig},
    direct::{content::extract_probe_content, target::DirectFetchTarget},
    error::AppError,
    net::SecureHttpClient,
};
use alloc::sync::Arc;
use futures::future::FutureExt as _;
use num_traits::ToPrimitive as _;
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
    let maximum_length = content.chars().count().max(probe_content.chars().count());
    let maximum_length_as_float = maximum_length
        .to_f64()
        .ok_or_else(|| AppError::internal("probe content length cannot be represented"))?;
    let maximum_distance = ((1.0_f64 - direct_config.similarity_threshold)
        * maximum_length_as_float)
        .floor()
        .to_usize()
        .ok_or_else(|| AppError::internal("probe distance cannot be represented"))?;
    let Some(distance) = rapidfuzz::distance::levenshtein::distance_with_args(
        content.chars(),
        probe_content.chars(),
        &rapidfuzz::distance::levenshtein::Args::default().score_cutoff(maximum_distance),
    ) else {
        return Ok(());
    };
    let similarity = if maximum_length == 0 {
        1.0_f64
    } else {
        let distance_as_float = distance
            .to_f64()
            .ok_or_else(|| AppError::internal("probe distance cannot be represented"))?;
        1.0_f64 - distance_as_float / maximum_length_as_float
    };
    if similarity >= direct_config.similarity_threshold {
        return Err(AppError::client(format!(
            "Direct Markdown content is too similar to a known-missing URL response ({similarity:.3})."
        )));
    }
    Ok(())
}
