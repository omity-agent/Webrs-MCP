use crate::{
    Result,
    config::{DirectFetchConfig, HttpConfig},
    direct::{content::extract_content, target::DirectFetchTarget},
    error::AppError,
    net::{FetchResponse, SecureHttpClient},
};
use futures::future::FutureExt as _;
#[inline]
pub(super) fn shared_probe_fetch(
    client: SecureHttpClient,
    probe_url: String,
    target: &DirectFetchTarget,
    direct_config: &DirectFetchConfig,
    http_config: &HttpConfig,
) -> Result<super::SharedProbeFetch> {
    let headers = super::request_headers(&client, target)?;
    let timeout_seconds = http_config.direct_fetch_timeout_seconds;
    let max_bytes = direct_config.max_bytes;
    Ok(async move {
        client
            .get_with_body_limit(&probe_url, headers, timeout_seconds, max_bytes)
            .await
    }
    .boxed()
    .shared())
}
pub(super) fn extract_direct_content(
    target: &DirectFetchTarget,
    response: &FetchResponse,
    probe_response: Option<Result<FetchResponse>>,
    direct_config: &DirectFetchConfig,
) -> Result<String> {
    let content = extract_content(
        target,
        response.status.as_u16(),
        &response.headers,
        &response.body,
        direct_config,
    )?;
    if response.status.as_u16() == 200
        && let Some(probe_result) = probe_response
    {
        let probe_check_response = probe_result?;
        reject_if_probe_is_similar(target, &probe_check_response, direct_config, &content)?;
    }
    Ok(content)
}
fn reject_if_probe_is_similar(
    target: &DirectFetchTarget,
    response: &FetchResponse,
    direct_config: &DirectFetchConfig,
    content: &str,
) -> Result<()> {
    let Some(probe_url) = target.similarity_probe_url.as_deref() else {
        return Ok(());
    };
    if response.status.as_u16() != 200 {
        return Ok(());
    }
    let mut probe_target = target.clone();
    #[expect(
        clippy::assigning_clones,
        reason = "The cloned probe URL replaces a cloned request target for one validation request."
    )]
    {
        probe_target.request_url = probe_url.to_owned();
    }
    probe_target.similarity_probe_url = None;
    let probe_content = extract_content(
        &probe_target,
        response.status.as_u16(),
        &response.headers,
        &response.body,
        direct_config,
    )?;
    let similarity = strsim::normalized_levenshtein(content, &probe_content);
    if similarity >= direct_config.similarity_threshold {
        return Err(AppError::client(format!(
            "Direct Markdown content is too similar to a known-missing URL response ({similarity:.3})."
        )));
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::extract_direct_content;
    use crate::{config, direct::DirectFetchTarget, net::FetchResponse};
    use reqwest::{StatusCode, header::HeaderMap};
    #[test]
    fn similar_probe_response_is_rejected() {
        let config = config::load_embedded().unwrap_or_else(|error| panic!("{error}"));
        let mut target =
            DirectFetchTarget::text("https://example.com/page", "https://example.com/page.md");
        target.similarity_probe_url = Some("https://example.com/missing.md".to_owned());
        let response = FetchResponse {
            status: StatusCode::OK,
            headers: HeaderMap::new(),
            body: b"generated missing page".to_vec(),
        };
        let error = extract_direct_content(
            &target,
            &response,
            Some(Ok(response.clone())),
            &config.direct_fetch,
        )
        .unwrap_err()
        .client_message();
        assert!(error.contains("too similar"));
    }
}
