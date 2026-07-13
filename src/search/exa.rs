use crate::{
    Result,
    config::{AppConfig, SearchConfig},
    error::{AppError, http_service_error},
    models::{SearchQueryRequest, SearchResult},
    net::{SecureHttpClient, secure_client_from_config},
};
use chrono::{Days, Utc};
use futures::future::try_join_all;
use reqwest::header::{CONTENT_TYPE, HeaderMap, HeaderValue};
use serde::{Deserialize, Serialize};
#[derive(Clone)]
pub struct ExaSearchClient {
    config: SearchConfig,
    timeout_seconds: f64,
    endpoint: String,
    http: SecureHttpClient,
}
#[derive(Serialize)]
struct ExaSearchPayload<'request> {
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
    contents: ExaContents<'request>,
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
#[derive(Deserialize)]
struct ExaSearchResponse {
    results: Vec<ExaResult>,
}
#[derive(Deserialize)]
struct ExaResult {
    title: Option<String>,
    #[serde(rename = "publishedDate", alias = "published_date")]
    published_date: Option<String>,
    url: String,
    highlights: Option<Vec<String>>,
}
impl ExaSearchClient {
    #[inline]
    pub fn new(config: &AppConfig) -> Result<Self> {
        Ok(Self {
            config: config.search.clone(),
            timeout_seconds: config.http.timeout_seconds,
            endpoint: config.search.endpoint.clone(),
            http: secure_client_from_config(config)?,
        })
    }
    #[expect(
        clippy::missing_inline_in_public_items,
        reason = "Search fan-out performs async HTTP I/O and is not an inline candidate."
    )]
    pub async fn search_many(
        &self,
        requests: &[SearchQueryRequest],
        api_key: &str,
    ) -> Result<Vec<SearchResult>> {
        let searches = requests
            .iter()
            .map(|request| self.search_one(request, api_key));
        let grouped = try_join_all(searches).await?;
        Ok(grouped.into_iter().flatten().collect())
    }
    async fn search_one(
        &self,
        request: &SearchQueryRequest,
        api_key: &str,
    ) -> Result<Vec<SearchResult>> {
        let body = sonic_rs::to_vec(&self.payload(request)).map_err(|error| {
            AppError::internal(format!("failed to encode Exa request: {error}"))
        })?;
        let response = self
            .http
            .post(
                &self.endpoint,
                Self::headers(api_key)?,
                body,
                self.timeout_seconds,
            )
            .await?;
        if response.status.as_u16() >= 400 {
            return Err(http_service_error(
                crate::search::provider_name(),
                response.status.as_u16(),
            ));
        }
        let payload: ExaSearchResponse = sonic_rs::from_slice(&response.body)
            .map_err(|_error| AppError::client("Exa returned malformed JSON."))?;
        Ok(payload.results.into_iter().map(to_search_result).collect())
    }
    fn payload<'request>(
        &'request self,
        request: &'request SearchQueryRequest,
    ) -> ExaSearchPayload<'request> {
        ExaSearchPayload {
            query: &request.q,
            search_type: &self.config.search_type,
            num_results: self.config.num_results,
            include_domains: normalize_domains(request.domains.as_deref()),
            start_published_date: start_published_date(request.recency),
            category: request
                .category
                .as_ref()
                .map(crate::models::SearchCategory::as_str),
            contents: ExaContents {
                highlights: ExaHighlights {
                    query: &request.q,
                    max_characters: self.config.highlights_max_characters,
                },
                max_age_hours: self.config.max_age_hours,
                livecrawl_timeout: self.config.livecrawl_timeout,
            },
        }
    }
    fn headers(api_key: &str) -> Result<HeaderMap> {
        let mut headers = HeaderMap::new();
        headers.insert(
            crate::search::api_key_header(),
            HeaderValue::from_str(api_key)
                .map_err(|error| AppError::internal(format!("invalid Exa key header: {error}")))?,
        );
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        Ok(headers)
    }
}
fn normalize_domains(domains: Option<&[String]>) -> Option<Vec<String>> {
    let normalized: Vec<String> = domains?
        .iter()
        .filter_map(|domain| normalize_domain(domain))
        .collect();
    (!normalized.is_empty()).then_some(normalized)
}
fn normalize_domain(domain: &str) -> Option<String> {
    let value = domain.trim();
    if value.is_empty() {
        return None;
    }
    let parse_input = if value.contains("://") {
        value.to_owned()
    } else {
        format!("https://{value}")
    };
    let parsed = url::Url::parse(&parse_input).ok()?;
    parsed.host_str().map(str::to_ascii_lowercase)
}
fn start_published_date(recency: Option<u64>) -> Option<String> {
    let days = Days::new(recency?);
    Utc::now()
        .checked_sub_days(days)
        .map(|date| date.date_naive().to_string())
}
fn to_search_result(result: ExaResult) -> SearchResult {
    SearchResult {
        title: result.title,
        date: result.published_date,
        url: result.url,
        highlight: result.highlights.unwrap_or_default().join("\n"),
    }
}
