use crate::{
    Result,
    config::AppConfig,
    mcp::processing::BlockingPool,
    mcp::schemas,
    net::secure_client_from_config,
    page::{PageFetcher, TokenChunker},
    search::SearchClient,
};
use alloc::sync::Arc;
use rmcp::model::Tool;
#[derive(Clone)]
pub(super) struct ToolRuntime {
    pub(super) config: Arc<AppConfig>,
    pub(super) chunker: TokenChunker,
    pub(super) processing: BlockingPool,
    pub(super) page_fetcher: PageFetcher,
    pub(super) search: SearchClient,
    pub(super) tools: Arc<[Tool]>,
}
impl ToolRuntime {
    pub(super) fn new(config: AppConfig, mode: crate::cli::WorkMode) -> Result<Self> {
        config.validate()?;
        let shared_config = Arc::new(config);
        let http = secure_client_from_config(&shared_config)?;
        let tools: Arc<[Tool]> = schemas::tools(&shared_config.tools, mode)?.into();
        Ok(Self {
            chunker: TokenChunker::new(&shared_config.chunking)?,
            processing: BlockingPool::new(shared_config.chunking.max_concurrent_tasks)?,
            page_fetcher: PageFetcher::new(Arc::clone(&shared_config), http.clone()),
            search: SearchClient::new(&shared_config, http),
            config: shared_config,
            tools,
        })
    }
    pub(super) fn tool(&self, name: &str) -> Option<Tool> {
        self.tools.iter().find(|tool| tool.name == name).cloned()
    }
    pub(super) fn tool_definitions(&self) -> Vec<Tool> {
        self.tools.iter().cloned().collect()
    }
}
