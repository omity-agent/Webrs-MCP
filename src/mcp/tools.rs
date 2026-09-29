use crate::{
    Result,
    arguments::{find_arguments, open_arguments, search_arguments},
    cli::WorkMode,
    config::AppConfig,
    error::AppError,
    mcp::processing::{BlockingPool, compile_patterns, find_pages, open_pages},
    models::SearchQueryResponse,
    net::secure_client_from_config,
    page::{PageFetcher, TokenChunker, reader::ReaderCredentials},
    search::{SearchClient, SearchCredentials},
};
use alloc::sync::Arc;
use axum::http::HeaderMap;
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
    config: Arc<AppConfig>,
    credentials: ToolCredentials,
    chunker: TokenChunker,
    processing: BlockingPool,
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
        let shared_config = Arc::new(config);
        let http = secure_client_from_config(&shared_config)?;
        Ok(Self {
            credentials,
            chunker: TokenChunker::new(&shared_config.chunking)?,
            processing: BlockingPool::new(shared_config.chunking.max_concurrent_tasks)?,
            page_fetcher: PageFetcher::new(Arc::clone(&shared_config), http.clone()),
            search: SearchClient::new(&shared_config, http),
            config: shared_config,
            mode,
        })
    }
    #[must_use]
    pub(crate) fn config(&self) -> &AppConfig {
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
            &self.processing,
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
        let patterns = compile_patterns(&normalized.value.requests, &self.processing).await?;
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
            &self.processing,
            self.config.find.clone(),
            self.config.chunking.chunk_tokens,
            warnings,
        )
        .await?;
        Ok(ToolOutput::Find(response))
    }
}
