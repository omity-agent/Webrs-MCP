use super::{SearchCredentials, exa, octen};
use crate::{
    Result,
    config::{AppConfig, SearchConfig},
    error::{AppError, http_service_error},
    models::{SearchQueryRequest, SearchResult},
    net::{SecureHttpClient, secure_client_from_config},
};
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
    pub warnings: Vec<String>,
}
impl SearchClient {
    #[inline]
    pub fn new(config: &AppConfig) -> Result<Self> {
        Ok(Self {
            config: config.search.clone(),
            timeout_seconds: config.http.timeout_seconds,
            http: secure_client_from_config(config)?,
        })
    }
    pub async fn search_many(
        &self,
        requests: &[SearchQueryRequest],
        credentials: &SearchCredentials,
        include_highlights: bool,
    ) -> Result<SearchBatch> {
        let headers = authentication(credentials)?;
        let pending = requests.iter().map(|request| {
            self.search_one(request, credentials, headers.clone(), include_highlights)
        });
        let responses = try_join_all(pending).await?;
        let mut batch = SearchBatch {
            groups: Vec::with_capacity(responses.len()),
            warnings: Vec::new(),
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
    async fn search_one(
        &self,
        request: &SearchQueryRequest,
        credentials: &SearchCredentials,
        headers: HeaderMap,
        include_highlights: bool,
    ) -> Result<(Vec<SearchResult>, Vec<String>)> {
        match *credentials {
            SearchCredentials::Exa(_) => {
                let payload = exa::payload(&self.config, request, include_highlights)?;
                let body = self
                    .send(&self.config.exa.endpoint, headers, &payload, "Exa")
                    .await?;
                Ok((exa::decode(&body)?, Vec::new()))
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
    ) -> Result<axum::body::Bytes>
    where
        Payload: Serialize + Sync,
    {
        let response = self
            .http
            .post_json(endpoint, headers, payload, self.timeout_seconds)
            .await?;
        if !response.status.is_success() {
            return Err(http_service_error(provider, response.status.as_u16()));
        }
        Ok(response.body)
    }
}
fn authentication(credentials: &SearchCredentials) -> Result<HeaderMap> {
    let key = credentials.api_key();
    if key.trim().is_empty() {
        return Err(AppError::client("Search API key must not be empty."));
    }
    let mut value = HeaderValue::from_str(key)
        .map_err(|error| AppError::client(format!("Invalid search API key header: {error}")))?;
    value.set_sensitive(true);
    let mut headers = HeaderMap::new();
    headers.insert("x-api-key", value);
    Ok(headers)
}
