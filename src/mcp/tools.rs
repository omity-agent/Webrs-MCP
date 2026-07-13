use crate::{
    Result,
    arguments::{find_arguments, open_arguments, search_arguments},
    config::AppConfig,
    error::AppError,
    mcp::processing::{find_pages, open_pages},
    models::SearchQueryResponse,
    page::{PageFetcher, TokenChunker, reader::ReaderCredentials},
    search::ExaSearchClient,
};
use alloc::borrow::Cow;
use axum::http::HeaderMap;
use fancy_regex::Regex;
use sonic_rs::Value;
#[cfg(test)]
mod tests;
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ToolCredentials {
    pub exa_api_key: Option<String>,
    pub reader: Option<ReaderCredentials>,
}
#[derive(Clone)]
pub struct ToolService {
    config: AppConfig,
    credentials: ToolCredentials,
    chunker: TokenChunker,
    page_fetcher: PageFetcher,
    search: ExaSearchClient,
}
impl ToolService {
    #[inline]
    pub fn new(config: AppConfig) -> Result<Self> {
        Self::new_with_credentials(config, ToolCredentials::default())
    }
    #[inline]
    pub fn new_with_credentials(config: AppConfig, credentials: ToolCredentials) -> Result<Self> {
        Ok(Self {
            credentials,
            chunker: TokenChunker::new(&config.chunking)?,
            page_fetcher: PageFetcher::new(config.clone())?,
            search: crate::search::client(&config)?,
            config,
        })
    }
    #[must_use]
    pub(crate) const fn config(&self) -> &AppConfig {
        &self.config
    }
    #[expect(
        clippy::missing_inline_in_public_items,
        reason = "Tool dispatch performs async service work and is not an inline candidate."
    )]
    pub async fn call(
        &self,
        name: &str,
        arguments: Option<Value>,
        headers: &HeaderMap,
    ) -> Result<Value> {
        match name {
            "search_query" => self.search_query(arguments, headers).await,
            "open" => self.open(arguments, headers).await,
            "find" => self.find(arguments, headers).await,
            other => Err(AppError::client(format!("Unknown tool: {other}"))),
        }
    }
    async fn search_query(&self, arguments: Option<Value>, headers: &HeaderMap) -> Result<Value> {
        let normalized = search_arguments(arguments)?;
        let key = required_api_key(
            headers,
            &self.config.headers.exa_api_key,
            self.credentials.exa_api_key.as_deref(),
        )?;
        let results = self
            .search
            .search_many(&normalized.value.requests, &key)
            .await?;
        to_value(&SearchQueryResponse {
            results,
            warning: normalized.warning,
        })
    }
    async fn open(&self, arguments: Option<Value>, headers: &HeaderMap) -> Result<Value> {
        let normalized = open_arguments(arguments)?;
        let warnings = normalized.warning.unwrap_or_default();
        let credentials = reader_credentials(
            headers,
            &self.config.headers,
            self.credentials.reader.clone(),
        )?;
        let urls = normalized
            .value
            .requests
            .iter()
            .map(|request| request.url.clone())
            .collect::<Vec<_>>();
        let pages = self
            .page_fetcher
            .fetch_many_partial(&urls, credentials.as_ref())
            .await;
        let response = open_pages(
            &normalized.value.requests,
            pages,
            self.chunker.clone(),
            warnings,
        )
        .await;
        to_value(&response)
    }
    async fn find(&self, arguments: Option<Value>, headers: &HeaderMap) -> Result<Value> {
        let normalized = find_arguments(arguments)?;
        let credentials = reader_credentials(
            headers,
            &self.config.headers,
            self.credentials.reader.clone(),
        )?;
        let patterns = compile_patterns(&normalized.value.requests)?;
        let urls = normalized
            .value
            .requests
            .iter()
            .map(|request| request.url.clone())
            .collect::<Vec<_>>();
        let pages = self
            .page_fetcher
            .fetch_many(&urls, credentials.as_ref())
            .await?;
        let warnings = normalized.warning.unwrap_or_default();
        let response = find_pages(
            &normalized.value.requests,
            pages,
            patterns,
            self.chunker.clone(),
            self.config.find.clone(),
            self.config.chunking.chunk_tokens,
            warnings,
        )
        .await?;
        to_value(&response)
    }
}
fn compile_patterns(requests: &[crate::models::FindRequest]) -> Result<Vec<Regex>> {
    requests
        .iter()
        .map(|request| {
            Regex::new(&format!("(?m){}", request.pattern)).map_err(|error| {
                AppError::client(format!(
                    "pattern is not a valid regular expression: {error}"
                ))
            })
        })
        .collect()
}
fn required_api_key<'key>(
    headers: &HeaderMap,
    name: &str,
    fallback: Option<&'key str>,
) -> Result<Cow<'key, str>> {
    if let Some(value) = optional_header(headers, name) {
        return Ok(Cow::Owned(value));
    }
    fallback
        .filter(|value| !value.is_empty())
        .map(Cow::Borrowed)
        .ok_or_else(|| AppError::client(format!("Missing required header: {name}.")))
}
fn reader_credentials(
    headers: &HeaderMap,
    config: &crate::config::HeaderConfig,
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
fn to_value<T>(value: &T) -> Result<Value>
where
    T: serde::Serialize,
{
    sonic_rs::to_value(value)
        .map_err(|error| AppError::internal(format!("failed to encode tool response: {error}")))
}
