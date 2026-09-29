use super::{filters, normalize_highlight};
use crate::{
    Result,
    config::SearchConfig,
    error::AppError,
    models::{SearchCategory, SearchQueryRequest, SearchResult},
};
use serde::{Deserialize, Serialize};
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ExaPayload<'request> {
    query: &'request str,
    #[serde(rename = "type")]
    search_type: &'request str,
    num_results: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    include_domains: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    start_published_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    category: Option<&'request str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    contents: Option<Contents<'request>>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Contents<'request> {
    highlights: Highlights<'request>,
    max_age_hours: u64,
    livecrawl_timeout: u32,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Highlights<'request> {
    query: &'request str,
    max_characters: u32,
}
#[derive(Deserialize)]
struct Response {
    results: Vec<Entry>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Entry {
    title: Option<String>,
    published_date: Option<String>,
    url: String,
    highlights: Option<Vec<String>>,
}
pub(super) fn payload<'request>(
    config: &'request SearchConfig,
    request: &'request SearchQueryRequest,
    include_highlights: bool,
) -> Result<ExaPayload<'request>> {
    Ok(ExaPayload {
        query: &request.q,
        search_type: &config.exa.search_type,
        num_results: config.num_results,
        include_domains: filters::domains(request.domains.as_deref())?,
        start_published_date: filters::since(request.recency)?
            .map(|date| date.date_naive().to_string()),
        category: request.category.as_ref().map(SearchCategory::as_str),
        contents: include_highlights.then_some(Contents {
            highlights: Highlights {
                query: &request.q,
                max_characters: config.exa.highlights_max_characters,
            },
            max_age_hours: config.exa.max_age_hours,
            livecrawl_timeout: config.exa.livecrawl_timeout,
        }),
    })
}
pub(super) fn decode(body: &[u8]) -> Result<Vec<SearchResult>> {
    let response: Response = sonic_rs::from_slice(body)
        .map_err(|error| AppError::client(format!("Exa returned malformed JSON: {error}")))?;
    Ok(response
        .results
        .into_iter()
        .map(|entry| SearchResult {
            title: entry.title,
            date: entry.published_date,
            url: entry.url,
            highlight: normalize_highlight(&entry.highlights.unwrap_or_default().join("\n")),
        })
        .collect())
}
