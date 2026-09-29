use super::identity::{reader_credentials, search_credentials};
use crate::{Result, config, error::AppError, page::reader::ReaderCredentials};
use axum::http::{
    HeaderMap, HeaderValue,
    header::{HeaderName, InvalidHeaderName},
};
#[test]
#[expect(
    clippy::panic_in_result_fn,
    reason = "The test uses assertions while Result keeps setup failures readable."
)]
fn reader_credentials_accepts_tinyfish_header() -> Result<()> {
    let config = config::load_embedded()?;
    let mut headers = HeaderMap::new();
    headers.insert(
        header_name(&config.headers.tinyfish_api_key)
            .map_err(|error| AppError::internal(format!("invalid test header name: {error}")))?,
        HeaderValue::from_static("tinyfish-key"),
    );
    assert_eq!(
        reader_credentials(&headers, &config.headers, None)?,
        Some(ReaderCredentials::TinyFish("tinyfish-key".to_owned()))
    );
    Ok(())
}
#[test]
fn reader_credentials_rejects_multiple_remote_reader_keys() {
    let config = config::load_embedded().unwrap_or_else(|error| panic!("{error}"));
    let mut headers = HeaderMap::new();
    headers.insert(
        header_name(&config.headers.jina_api_key).unwrap_or_else(|error| panic!("{error}")),
        HeaderValue::from_static("jina-key"),
    );
    headers.insert(
        header_name(&config.headers.tinyfish_api_key).unwrap_or_else(|error| panic!("{error}")),
        HeaderValue::from_static("tinyfish-key"),
    );
    let error = reader_credentials(&headers, &config.headers, None)
        .unwrap_err()
        .client_message();
    assert!(error.contains("not both"));
}
#[test]
fn reader_credentials_uses_fallback_without_reader_headers() {
    let config = config::load_embedded().unwrap_or_else(|error| panic!("{error}"));
    let headers = HeaderMap::new();
    assert_eq!(
        reader_credentials(
            &headers,
            &config.headers,
            Some(ReaderCredentials::Jina("jina-key".to_owned()))
        )
        .unwrap_or_else(|error| panic!("{error}")),
        Some(ReaderCredentials::Jina("jina-key".to_owned()))
    );
}
#[test]
fn missing_search_api_key_is_reported_when_tool_is_called() {
    let config = config::load_embedded().unwrap_or_else(|error| panic!("{error}"));
    let headers = HeaderMap::new();
    let error = search_credentials(&headers, &config.headers, None)
        .unwrap_err()
        .client_message();
    assert!(error.contains("Missing required search API key"));
    assert!(error.contains(&config.headers.exa_api_key));
}
fn header_name(value: &str) -> core::result::Result<HeaderName, InvalidHeaderName> {
    HeaderName::from_bytes(value.as_bytes())
}
