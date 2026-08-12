use super::exa::{normalize_domains, start_published_date};
use crate::{
    config::SearchConfig,
    models::{SearchCategory, SearchQueryRequest},
};
use serde::Serialize;
#[derive(Serialize)]
pub(super) struct ExaSearchPayload<'request> {
    query: &'request str,
    #[serde(rename = "type")]
    search_type: &'request str,
    #[serde(rename = "numResults")]
    num_results: u32,
    #[serde(rename = "includeDomains", skip_serializing_if = "Option::is_none")]
    include_domains: Option<Vec<String>>,
    #[serde(rename = "startPublishedDate", skip_serializing_if = "Option::is_none")]
    start_published_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    category: Option<&'request str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    contents: Option<ExaContents<'request>>,
}
#[derive(Serialize)]
struct ExaContents<'request> {
    highlights: ExaHighlights<'request>,
    #[serde(rename = "maxAgeHours")]
    max_age_hours: u64,
    #[serde(rename = "livecrawlTimeout")]
    livecrawl_timeout: u32,
}
#[derive(Serialize)]
struct ExaHighlights<'request> {
    query: &'request str,
    #[serde(rename = "maxCharacters")]
    max_characters: u32,
}
pub(super) fn exa_payload<'request>(
    config: &'request SearchConfig,
    request: &'request SearchQueryRequest,
    include_highlights: bool,
) -> ExaSearchPayload<'request> {
    ExaSearchPayload {
        query: &request.q,
        search_type: &config.search_type,
        num_results: config.num_results,
        include_domains: normalize_domains(request.domains.as_deref()),
        start_published_date: start_published_date(request.recency),
        category: request.category.as_ref().map(SearchCategory::as_str),
        contents: include_highlights.then_some(ExaContents {
            highlights: ExaHighlights {
                query: &request.q,
                max_characters: config.highlights_max_characters,
            },
            max_age_hours: config.max_age_hours,
            livecrawl_timeout: config.livecrawl_timeout,
        }),
    }
}
#[cfg(test)]
mod tests {
    use super::exa_payload;
    use crate::{config, models::SearchQueryRequest};
    #[test]
    fn filesystem_payload_omits_highlights() {
        let app_config = config::load_embedded().unwrap_or_else(|error| panic!("{error}"));
        let request = SearchQueryRequest {
            q: "query".to_owned(),
            recency: None,
            domains: None,
            category: None,
        };
        let payload = sonic_rs::to_string(&exa_payload(&app_config.search, &request, false))
            .unwrap_or_else(|error| panic!("failed to encode payload: {error}"));
        assert!(!payload.contains("\"contents\""));
    }
}
