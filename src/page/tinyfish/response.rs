use crate::{Result, error::AppError, error::http_service_error};
use serde::Deserialize;
use std::collections::HashMap;
#[derive(Deserialize)]
struct TinyFishResponse {
    results: Vec<TinyFishResult>,
    errors: Vec<TinyFishPageError>,
}
#[derive(Deserialize)]
struct TinyFishResult {
    url: String,
    text: String,
}
#[derive(Deserialize)]
struct TinyFishPageError {
    url: String,
    error: String,
    status: Option<u16>,
}
#[derive(Deserialize)]
struct TinyFishServiceError {
    error: TinyFishServiceErrorDetails,
    request_id: Option<String>,
}
#[derive(Deserialize)]
struct TinyFishServiceErrorDetails {
    code: String,
    message: String,
}
pub(super) fn extract_markdowns(urls: &[String], body: &[u8]) -> Result<Vec<Result<String>>> {
    let payload = sonic_rs::from_slice::<TinyFishResponse>(body).map_err(|error| {
        AppError::client(format!(
            "TinyFish returned an unsupported response: {error}"
        ))
    })?;
    let results = payload
        .results
        .into_iter()
        .map(|result| (result.url, result.text))
        .collect::<HashMap<_, _>>();
    let errors = payload
        .errors
        .into_iter()
        .map(|error| (error.url.clone(), error))
        .collect::<HashMap<_, _>>();
    let markdowns: Vec<Result<String>> = urls
        .iter()
        .map(|url| match (results.get(url), errors.get(url)) {
            (Some(text), _) => Ok(text.clone()),
            (None, Some(error)) => Err(page_error(error)),
            (None, None) => Err(AppError::client(format!(
                "TinyFish returned no content for the requested URL: {url}."
            ))),
        })
        .collect();
    Ok(markdowns)
}
pub(super) fn service_error(status: u16, body: &[u8]) -> AppError {
    let Ok(payload) = sonic_rs::from_slice::<TinyFishServiceError>(body) else {
        return http_service_error("TinyFish", status);
    };
    let request_id = payload
        .request_id
        .map_or_else(String::new, |value| format!(" Request ID: {value}."));
    AppError::client(format!(
        "TinyFish returned HTTP {status}: {}: {}.{request_id}",
        payload.error.code,
        payload.error.message.trim_end_matches('.')
    ))
}
fn page_error(error: &TinyFishPageError) -> AppError {
    let status = error
        .status
        .map_or_else(String::new, |value| format!(" with HTTP {value}"));
    AppError::client(format!(
        "TinyFish could not fetch {}: {}{}.",
        error.url, error.error, status
    ))
}
