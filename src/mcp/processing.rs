#![expect(
    clippy::pub_use,
    reason = "The shared executor is exposed for reuse and integration testing."
)]
use crate::{
    Result,
    config::{ChunkingConfig, FindConfig},
    error::AppError,
    models::{FindPage, FindRequest, FindResponse},
    page::{PageContent, TokenChunker, find_in_page},
    warnings::Warnings,
};
use fancy_regex::Regex;
use futures::{StreamExt as _, stream};
mod budget;
mod opening;
pub use budget::BlockingPool;
pub(crate) use opening::open_pages;
pub(crate) async fn find_pages(
    requests: &[FindRequest],
    pages: Vec<PageContent>,
    patterns: Vec<Regex>,
    chunker: TokenChunker,
    pool: &BlockingPool,
    config: FindConfig,
    chunking: &ChunkingConfig,
    mut warnings: Warnings,
) -> Result<FindResponse> {
    let max_concurrent_tasks = chunker.max_concurrent_tasks();
    let enabled = chunking.enabled;
    let snippet_limit = enabled.then_some(chunking.chunk_tokens);
    let tasks = requests
        .iter()
        .cloned()
        .zip(pages)
        .zip(patterns)
        .enumerate()
        .map(
            |(request_index, ((request, page), pattern)): (
                usize,
                ((FindRequest, PageContent), Regex),
            )| {
                let page_chunker = chunker.clone();
                let find_config = config.clone();
                let requested_snippet_tokens = request.snippet_tokens;
                async move {
                    pool.run(move || {
                        let mut page_warnings = Warnings::new();
                        let snippet_tokens = snippet_tokens_for_request(
                            requested_snippet_tokens,
                            request_index,
                            &mut page_warnings,
                            snippet_limit,
                            find_config.default_snippet_tokens,
                        );
                        let found = find_in_page(
                            &page,
                            &pattern,
                            snippet_tokens,
                            &page_chunker,
                            &find_config,
                            enabled,
                        )?;
                        Ok::<(FindPage, Warnings), AppError>((found, page_warnings))
                    })
                    .await
                }
            },
        );
    let joined = stream::iter(tasks)
        .buffered(max_concurrent_tasks)
        .collect::<Vec<_>>()
        .await;
    let mut found = Vec::with_capacity(joined.len());
    for task in joined {
        let (page, page_warnings) = task?;
        found.push(page);
        warnings.append(page_warnings);
    }
    Ok(FindResponse {
        pages: found,
        warning: warnings.into_option(),
    })
}
pub(crate) async fn compile_patterns(
    requests: &[FindRequest],
    pool: &BlockingPool,
) -> Result<Vec<Regex>> {
    let patterns = requests
        .iter()
        .map(|request| request.pattern.clone())
        .collect::<Vec<_>>();
    pool.run(move || {
        patterns
            .iter()
            .map(|pattern| {
                Regex::new(&format!("(?m){pattern}")).map_err(|error| {
                    AppError::client(format!(
                        "pattern is not a valid regular expression: {error}"
                    ))
                })
            })
            .collect()
    })
    .await
}
fn snippet_tokens_for_request(
    requested: Option<usize>,
    request_index: usize,
    warnings: &mut Warnings,
    chunk_limit: Option<usize>,
    default_snippet_tokens: usize,
) -> usize {
    let Some(value) = requested else {
        return default_snippet_tokens;
    };
    if let Some(limit) = chunk_limit
        && value > limit
    {
        warnings.push(format!(
            "\"requests[{request_index}].snippet_tokens\" exceeds chunk_tokens ({limit}); using {limit}"
        ));
        return limit;
    }
    value
}
