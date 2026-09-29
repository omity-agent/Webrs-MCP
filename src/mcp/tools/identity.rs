use crate::{
    Result, config::HeaderConfig, error::AppError, page::reader::ReaderCredentials,
    search::SearchCredentials,
};
use axum::http::HeaderMap;
pub(super) fn search_credentials(
    headers: &HeaderMap,
    config: &HeaderConfig,
    defaults: Option<&SearchCredentials>,
) -> Result<SearchCredentials> {
    let exa = search_header(headers, &config.exa_api_key)?;
    let octen = search_header(headers, &config.octen_api_key)?;
    match (exa, octen) {
        (Some(_exa), Some(_octen)) => Err(AppError::client(format!(
            "Provide exactly one search API key header: {} or {}, not both.",
            config.exa_api_key, config.octen_api_key
        ))),
        (Some(key), None) => Ok(SearchCredentials::Exa(key)),
        (None, Some(key)) => Ok(SearchCredentials::Octen(key)),
        (None, None) => defaults.cloned().ok_or_else(|| {
            AppError::client(format!(
                "Missing required search API key: {} or {}.",
                config.exa_api_key, config.octen_api_key
            ))
        }),
    }
}
fn search_header(headers: &HeaderMap, name: &str) -> Result<Option<String>> {
    let mut values = headers.get_all(name).iter();
    let Some(value) = values.next() else {
        return Ok(None);
    };
    if values.next().is_some() {
        return Err(AppError::client(format!(
            "Duplicate API key header: {name}."
        )));
    }
    let text = value
        .to_str()
        .map_err(|error| AppError::client(format!("Invalid {name} header: {error}")))?;
    if text.trim().is_empty() {
        return Err(AppError::client(format!("Empty API key header: {name}.")));
    }
    Ok(Some(text.to_owned()))
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
