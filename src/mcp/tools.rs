use crate::{
    Result,
    arguments::{find_arguments, open_arguments, search_arguments},
    cli::WorkMode,
    config::{AppConfig, ToolKind},
    error::AppError,
    mcp::processing::{compile_patterns, find_pages, open_pages},
    models::SearchQueryResponse,
    page::reader::ReaderCredentials,
    search::SearchCredentials,
};
use alloc::sync::Arc;
use axum::http::HeaderMap;
use sonic_rs::Value;
mod filesystem;
mod identity;
mod render;
mod runtime;
use identity::{reader_credentials, search_credentials};
pub(crate) use render::ToolOutput;
use runtime::ToolRuntime;
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ToolCredentials {
    pub search: Option<SearchCredentials>,
    pub reader: Option<ReaderCredentials>,
}
#[derive(Clone)]
pub struct ToolService {
    runtime: Arc<ToolRuntime>,
    credentials: ToolCredentials,
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
            runtime: Arc::new(ToolRuntime::new(config, mode)?),
            credentials,
            mode,
        })
    }
    #[must_use]
    pub(crate) fn config(&self) -> &AppConfig {
        &self.runtime.config
    }
    #[must_use]
    pub(crate) fn tool(&self, name: &str) -> Option<rmcp::model::Tool> {
        self.runtime.tool(name)
    }
    #[must_use]
    pub(crate) fn has_tool(&self, name: &str) -> bool {
        self.runtime.tools.iter().any(|tool| tool.name == name)
    }
    #[must_use]
    pub(crate) fn tool_definitions(&self) -> Vec<rmcp::model::Tool> {
        self.runtime.tool_definitions()
    }
    pub(crate) async fn call(
        &self,
        name: &str,
        arguments: Option<Value>,
        headers: &HeaderMap,
    ) -> Result<ToolOutput> {
        let kind = self
            .runtime
            .config
            .tools
            .resolve(name)
            .ok_or_else(|| AppError::client(format!("Unknown tool: {name}")))?;
        if self.mode == WorkMode::Filesystem {
            return Ok(self.filesystem_call(kind, arguments, headers).await);
        }
        match kind {
            ToolKind::Search => self.search_query(arguments, headers).await,
            ToolKind::Open => self.open(arguments, headers).await,
            ToolKind::Find => self.find(arguments, headers).await,
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
            &self.runtime.config.headers,
            self.credentials.search.as_ref(),
        )?;
        let batch = self
            .runtime
            .search
            .search_many(&normalized.value.requests, &credentials, true)
            .await?;
        let mut warnings = normalized.warnings;
        warnings.append(batch.warnings);
        Ok(ToolOutput::Search(SearchQueryResponse {
            results: batch.groups.into_iter().flatten().collect(),
            warning: warnings.into_option(),
        }))
    }
    async fn open(&self, arguments: Option<Value>, headers: &HeaderMap) -> Result<ToolOutput> {
        let normalized = open_arguments(arguments)?;
        let warnings = normalized.warnings;
        let credentials = reader_credentials(
            headers,
            &self.runtime.config.headers,
            self.credentials.reader.clone(),
        )?;
        let urls = normalized
            .value
            .requests
            .iter()
            .map(|request| request.url.clone())
            .collect::<Vec<_>>();
        let pages = self
            .runtime
            .page_fetcher
            .fetch_many_partial(&urls, credentials.as_ref())
            .await;
        let response = open_pages(
            &normalized.value.requests,
            pages,
            self.runtime.chunker.clone(),
            &self.runtime.processing,
            warnings,
        )
        .await;
        Ok(ToolOutput::Open(response))
    }
    async fn find(&self, arguments: Option<Value>, headers: &HeaderMap) -> Result<ToolOutput> {
        let normalized = find_arguments(arguments)?;
        let credentials = reader_credentials(
            headers,
            &self.runtime.config.headers,
            self.credentials.reader.clone(),
        )?;
        let patterns =
            compile_patterns(&normalized.value.requests, &self.runtime.processing).await?;
        let urls = normalized
            .value
            .requests
            .iter()
            .map(|request| request.url.clone())
            .collect::<Vec<_>>();
        let pages = self
            .runtime
            .page_fetcher
            .fetch_many(&urls, credentials.as_ref())
            .await?;
        let warnings = normalized.warnings;
        let response = find_pages(
            &normalized.value.requests,
            pages,
            patterns,
            self.runtime.chunker.clone(),
            &self.runtime.processing,
            self.runtime.config.find.clone(),
            self.runtime.config.chunking.chunk_tokens,
            warnings,
        )
        .await?;
        Ok(ToolOutput::Find(response))
    }
}
