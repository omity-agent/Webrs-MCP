use super::{SearchCredentials, exa, octen};
use crate::{
    Result,
    config::{AppConfig, SearchConfig},
    error::AppError,
    models::{SearchQueryRequest, SearchResult},
    net::SecureHttpClient,
    warnings::Warnings,
};
use bytes::Bytes;
use futures::future::try_join_all;
use reqwest::header::{HeaderMap, HeaderValue};
use serde::Serialize;
#[derive(Clone)]
pub struct SearchClient {
    config: SearchConfig,
    timeout_seconds: f64,
    http: SecureHttpClient,
}
pub struct SearchBatch {
    pub groups: Vec<Vec<SearchResult>>,
    pub warnings: Warnings,
}
impl SearchClient {
    #[inline]
    #[must_use]
    pub fn new(config: &AppConfig, http: SecureHttpClient) -> Self {
        Self {
            config: config.search.clone(),
            timeout_seconds: config.http.timeout_seconds,
            http,
        }
    }
    pub async fn search_many(
        &self,
        requests: &[SearchQueryRequest],
        credentials: &SearchCredentials,
        include_highlights: bool,
    ) -> Result<SearchBatch> {
        let headers = self.headers(credentials, include_highlights)?;
        let pending = requests.iter().map(|request| {
            self.search_one(request, credentials, headers.clone(), include_highlights)
        });
        let responses = try_join_all(pending).await?;
        let mut batch = SearchBatch {
            groups: Vec::with_capacity(responses.len()),
            warnings: Warnings::new(),
        };
        for (index, (results, warnings)) in responses.into_iter().enumerate() {
            batch.groups.push(results);
            batch.warnings.extend(
                warnings
                    .into_iter()
                    .map(|warning| format!("requests[{index}]: {warning}")),
            );
        }
        Ok(batch)
    }
    fn headers(
        &self,
        credentials: &SearchCredentials,
        include_highlights: bool,
    ) -> Result<HeaderMap> {
        let key = credentials.api_key();
        if key.trim().is_empty() {
            return Err(AppError::client("Search API key must not be empty."));
        }
        let mut value = HeaderValue::from_str(key)
            .map_err(|error| AppError::client(format!("Invalid search API key header: {error}")))?;
        value.set_sensitive(true);
        let mut headers = HeaderMap::new();
        headers.insert("x-api-key", value);
        if include_highlights && matches!(credentials, SearchCredentials::Exa(_)) {
            let beta = HeaderValue::from_str(&self.config.exa.dynamic_highlights_beta).map_err(
                |error| AppError::config(format!("search.exa.dynamic_highlights_beta: {error}")),
            )?;
            headers.insert("exa-beta", beta);
        }
        Ok(headers)
    }
    async fn search_one(
        &self,
        request: &SearchQueryRequest,
        credentials: &SearchCredentials,
        headers: HeaderMap,
        include_highlights: bool,
    ) -> Result<(Vec<SearchResult>, Warnings)> {
        match *credentials {
            SearchCredentials::Exa(_) => {
                let payload = exa::payload(&self.config, request, include_highlights)?;
                let body = self
                    .send(&self.config.exa.endpoint, headers, &payload, "Exa")
                    .await?;
                Ok((exa::decode(&body)?, Warnings::new()))
            }
            SearchCredentials::Octen(_) => {
                let payload = octen::payload(&self.config, request, include_highlights)?;
                let body = self
                    .send(&self.config.octen.endpoint, headers, &payload, "Octen")
                    .await?;
                let (results, mut warnings) = octen::decode(&body)?;
                if request.category.is_some() {
                    warnings.push(
                        "Octen does not support category; the filter was ignored.".to_owned(),
                    );
                }
                Ok((results, warnings))
            }
        }
    }
    async fn send<Payload>(
        &self,
        endpoint: &str,
        headers: HeaderMap,
        payload: &Payload,
        provider: &str,
    ) -> Result<Bytes>
    where
        Payload: Serialize + Sync,
    {
        let response = self
            .http
            .post_json_checked(endpoint, headers, payload, self.timeout_seconds, provider)
            .await?;
        Ok(response.body)
    }
}
