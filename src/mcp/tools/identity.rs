use crate::{Result, config::HeaderConfig, error::AppError, page::reader::ReaderCredentials};
use alloc::borrow::Cow;
use axum::http::HeaderMap;
pub(super) fn required_api_key<'key>(
    headers: &HeaderMap,
    name: &str,
    fallback: Option<&'key str>,
) -> Result<Cow<'key, str>> {
    if let Some(value) = optional_header(headers, name) {
        return Ok(Cow::Owned(value));
    }
    fallback
        .filter(|value| !value.is_empty())
        .map(Cow::Borrowed)
        .ok_or_else(|| AppError::client(format!("Missing required header: {name}.")))
}
pub(super) fn reader_credentials(
    headers: &HeaderMap,
    config: &HeaderConfig,
    fallback: Option<ReaderCredentials>,
) -> Result<Option<ReaderCredentials>> {
    let jina = optional_header(headers, &config.jina_api_key);
    let tinyfish = optional_header(headers, &config.tinyfish_api_key);
    match (jina, tinyfish) {
        (Some(_jina), Some(_tinyfish)) => Err(AppError::client(format!(
            "Provide exactly one remote reader API key header: {} or {}, not both.",
            config.jina_api_key, config.tinyfish_api_key
        ))),
        (Some(api_key), None) => Ok(Some(ReaderCredentials::Jina(api_key))),
        (None, Some(api_key)) => Ok(Some(ReaderCredentials::TinyFish(api_key))),
        (None, None) => Ok(fallback),
    }
}
fn optional_header(headers: &HeaderMap, name: &str) -> Option<String> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}
