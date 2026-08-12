use super::payload::exa_payload;
use crate::{
    Result,
    config::{AppConfig, SearchConfig},
    error::{AppError, http_service_error},
    models::{SearchQueryRequest, SearchResult},
    net::{SecureHttpClient, secure_client_from_config},
};
use chrono::{Days, Utc};
use futures::future::try_join_all;
use reqwest::header::{HeaderMap, HeaderValue};
use serde::Deserialize;
#[derive(Clone)]
pub struct ExaSearchClient {
    config: SearchConfig,
    timeout_seconds: f64,
    endpoint: String,
    http: SecureHttpClient,
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
        let grouped = self.search_grouped(requests, api_key, true).await?;
        Ok(grouped.into_iter().flatten().collect())
    }
    pub(crate) async fn search_urls_many(
        &self,
        requests: &[SearchQueryRequest],
        api_key: &str,
    ) -> Result<Vec<Vec<String>>> {
        let grouped = self.search_grouped(requests, api_key, false).await?;
        Ok(grouped
            .into_iter()
            .map(|results| results.into_iter().map(|result| result.url).collect())
            .collect())
    }
    async fn search_grouped(
        &self,
        requests: &[SearchQueryRequest],
        api_key: &str,
        include_highlights: bool,
    ) -> Result<Vec<Vec<SearchResult>>> {
        let searches = requests
            .iter()
            .map(|request| self.search_one(request, api_key, include_highlights));
        try_join_all(searches).await
    }
    async fn search_one(
        &self,
        request: &SearchQueryRequest,
        api_key: &str,
        include_highlights: bool,
    ) -> Result<Vec<SearchResult>> {
        let request_payload = exa_payload(&self.config, request, include_highlights);
        let response = self
            .http
            .post_json(
                &self.endpoint,
                Self::headers(api_key)?,
                &request_payload,
                self.timeout_seconds,
            )
            .await?;
        if response.status.as_u16() >= 400 {
            return Err(http_service_error(
                crate::search::provider_name(),
                response.status.as_u16(),
            ));
        }
        let response_payload: ExaSearchResponse = sonic_rs::from_slice(&response.body)
            .map_err(|_error| AppError::client("Exa returned malformed JSON."))?;
        Ok(response_payload
            .results
            .into_iter()
            .map(to_search_result)
            .collect())
    }
    fn headers(api_key: &str) -> Result<HeaderMap> {
        let mut headers = HeaderMap::new();
        headers.insert(
            crate::search::api_key_header(),
            HeaderValue::from_str(api_key)
                .map_err(|error| AppError::internal(format!("invalid Exa key header: {error}")))?,
        );
        Ok(headers)
    }
}
pub(super) fn normalize_domains(domains: Option<&[String]>) -> Option<Vec<String>> {
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
pub(super) fn start_published_date(recency: Option<u64>) -> Option<String> {
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
