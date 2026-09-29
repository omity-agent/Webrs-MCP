use crate::{
    Result,
    arguments::{find_arguments, open_arguments, search_arguments},
    cli::WorkMode,
    config::AppConfig,
    error::AppError,
    mcp::processing::{find_pages, open_pages},
    models::SearchQueryResponse,
    page::{PageFetcher, TokenChunker, reader::ReaderCredentials},
    search::{SearchClient, SearchCredentials},
};
use axum::http::HeaderMap;
use fancy_regex::Regex;
use sonic_rs::Value;
mod filesystem;
mod identity;
mod render;
#[cfg(test)]
mod tests;
use identity::{reader_credentials, search_credentials};
pub(crate) use render::ToolOutput;
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ToolCredentials {
    pub search: Option<SearchCredentials>,
    pub reader: Option<ReaderCredentials>,
}
#[derive(Clone)]
pub struct ToolService {
    config: AppConfig,
    credentials: ToolCredentials,
    chunker: TokenChunker,
    page_fetcher: PageFetcher,
    search: SearchClient,
    mode: WorkMode,
}
impl ToolService {
    #[inline]
    pub fn new(config: AppConfig) -> Result<Self> {
        Self::new_with_credentials(config, ToolCredentials::default(), WorkMode::Response)
    }
    #[inline]
    pub fn new_with_credentials(
        config: AppConfig,
        credentials: ToolCredentials,
        mode: WorkMode,
    ) -> Result<Self> {
        Ok(Self {
            credentials,
            chunker: TokenChunker::new(&config.chunking)?,
            page_fetcher: PageFetcher::new(config.clone())?,
            search: SearchClient::new(&config)?,
            config,
            mode,
        })
    }
    #[must_use]
    pub(crate) const fn config(&self) -> &AppConfig {
        &self.config
    }
    #[must_use]
    pub(crate) const fn mode(&self) -> WorkMode {
        self.mode
    }
    pub(crate) async fn call(
        &self,
        name: &str,
        arguments: Option<Value>,
        headers: &HeaderMap,
    ) -> Result<ToolOutput> {
        if self.mode == WorkMode::Filesystem {
            return Ok(self.filesystem_call(name, arguments, headers).await);
        }
        match name {
            "search_query" => self.search_query(arguments, headers).await,
            "open" => self.open(arguments, headers).await,
            "find" => self.find(arguments, headers).await,
            other => Err(AppError::client(format!("Unknown tool: {other}"))),
        }
    }
    async fn search_query(
        &self,
        arguments: Option<Value>,
        headers: &HeaderMap,
    ) -> Result<ToolOutput> {
        let normalized = search_arguments(arguments)?;
        let credentials = search_credentials(
            headers,
            &self.config.headers,
            self.credentials.search.as_ref(),
        )?;
        let batch = self
            .search
            .search_many(&normalized.value.requests, &credentials, true)
            .await?;
        let mut warnings = normalized.warning.unwrap_or_default();
        warnings.extend(batch.warnings);
        Ok(ToolOutput::Search(SearchQueryResponse {
            results: batch.groups.into_iter().flatten().collect(),
            warning: (!warnings.is_empty()).then_some(warnings),
        }))
    }
    async fn open(&self, arguments: Option<Value>, headers: &HeaderMap) -> Result<ToolOutput> {
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
        Ok(ToolOutput::Open(response))
    }
    async fn find(&self, arguments: Option<Value>, headers: &HeaderMap) -> Result<ToolOutput> {
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
        Ok(ToolOutput::Find(response))
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
