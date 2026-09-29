use super::{filters, normalize_highlight};
use crate::{
    Result,
    config::SearchConfig,
    error::AppError,
    models::{SearchQueryRequest, SearchResult},
};
use chrono::SecondsFormat;
use serde::{Deserialize, Serialize};
#[derive(Serialize)]
pub(super) struct OctenPayload<'request> {
    query: &'request str,
    count: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    include_domains: Option<Vec<String>>,
    time_basis: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    start_time: Option<String>,
    highlight: Highlight,
    full_content: FullContent,
    format: &'request str,
    safesearch: &'request str,
}
#[derive(Serialize)]
struct Highlight {
    enable: bool,
    max_tokens: u32,
}
#[derive(Serialize)]
struct FullContent {
    enable: bool,
}
#[derive(Deserialize)]
struct Response {
    code: i64,
    msg: String,
    request_id: Option<String>,
    data: Option<Data>,
    meta: Option<Meta>,
}
#[derive(Deserialize)]
struct Data {
    results: Vec<Entry>,
}
#[derive(Deserialize)]
struct Meta {
    warning: Option<String>,
}
#[derive(Deserialize)]
struct Entry {
    title: Option<String>,
    url: String,
    highlight: Option<String>,
    time_published: Option<String>,
}
pub(super) fn payload<'request>(
    config: &'request SearchConfig,
    request: &'request SearchQueryRequest,
    include_highlights: bool,
) -> Result<OctenPayload<'request>> {
    if request.q.chars().count() > 500 {
        return Err(AppError::client(
            "Octen query exceeds the 500-character limit.",
        ));
    }
    let include_domains = filters::domains(request.domains.as_deref())?;
    if let Some(domains) = include_domains.as_ref()
        && (domains.len() > 1200 || domains.iter().any(|domain| domain.len() > 60))
    {
        return Err(AppError::client(
            "Octen accepts at most 1200 domains of at most 60 characters each.",
        ));
    }
    Ok(OctenPayload {
        query: &request.q,
        count: config.num_results,
        include_domains,
        time_basis: "published",
        start_time: filters::since(request.recency)?
            .map(|date| date.to_rfc3339_opts(SecondsFormat::Secs, true)),
        highlight: Highlight {
            enable: include_highlights,
            max_tokens: config.octen.highlights_max_tokens,
        },
        full_content: FullContent { enable: false },
        format: &config.octen.format,
        safesearch: &config.octen.safesearch,
    })
}
pub(super) fn decode(body: &[u8]) -> Result<(Vec<SearchResult>, Vec<String>)> {
    let response: Response = sonic_rs::from_slice(body)
        .map_err(|error| AppError::client(format!("Octen returned malformed JSON: {error}")))?;
    if response.code != 0 {
        return Err(AppError::client(format!(
            "Octen returned code {}: {} (request_id: {}).",
            response.code,
            response.msg,
            response.request_id.as_deref().unwrap_or("not provided")
        )));
    }
    let data = response
        .data
        .ok_or_else(|| AppError::client("Octen success response is missing data."))?;
    let warnings = response
        .meta
        .and_then(|meta| meta.warning)
        .filter(|warning| !warning.trim().is_empty())
        .map(|warning| format!("Octen: {warning}"))
        .into_iter()
        .collect();
    let results = data
        .results
        .into_iter()
        .map(|entry| SearchResult {
            title: entry.title,
            date: entry.time_published,
            url: entry.url,
            highlight: normalize_highlight(entry.highlight.as_deref().unwrap_or_default()),
        })
        .collect();
    Ok((results, warnings))
}
