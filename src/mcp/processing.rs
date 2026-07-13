use crate::{
    Result,
    config::FindConfig,
    error::AppError,
    models::{FindPage, FindRequest, FindResponse, OpenRequest, OpenResponse, OpenResult},
    page::{PageContent, TokenChunker, find_in_page, open_page_chunk},
};
use fancy_regex::Regex;
use futures::future::{join_all, try_join_all};
#[cfg(test)]
mod tests;
pub(crate) async fn open_pages(
    requests: &[OpenRequest],
    pages: Vec<Result<PageContent>>,
    chunker: TokenChunker,
    mut warnings: Vec<String>,
) -> OpenResponse {
    let mut fetched_pages = pages.into_iter();
    let tasks = requests.iter().enumerate().map(|(request_index, request)| {
        let fetched = fetched_pages
            .next()
            .unwrap_or_else(|| Err(AppError::internal("page fetch result was missing")));
        let page_chunker = chunker.clone();
        let chunk_index = request.chunk;
        let task_url = request.url.clone();
        let result_url = task_url.clone();
        async move {
            let task = tokio::task::spawn_blocking(move || {
                process_open_result(task_url, fetched, chunk_index, request_index, &page_chunker)
            });
            task.await.unwrap_or_else(|error| {
                (
                    OpenResult::Failure {
                        url: result_url,
                        error: AppError::internal(format!("page processing task failed: {error}"))
                            .client_message(),
                    },
                    Vec::new(),
                )
            })
        }
    });
    let joined = join_all(tasks).await;
    let mut results = Vec::with_capacity(joined.len());
    for (result, page_warnings) in joined {
        results.push(result);
        warnings.extend(page_warnings);
    }
    OpenResponse {
        results,
        warning: (!warnings.is_empty()).then_some(warnings),
    }
}
fn process_open_result(
    url: String,
    fetched: Result<PageContent>,
    chunk_index: usize,
    request_index: usize,
    chunker: &TokenChunker,
) -> (OpenResult, Vec<String>) {
    let mut warnings = Vec::new();
    let opened = fetched.and_then(|page| {
        open_page_chunk(&page, chunk_index, request_index, chunker, &mut warnings)
    });
    let result = match opened {
        Ok(page) => OpenResult::Success { url, page },
        Err(error) => OpenResult::Failure {
            url,
            error: error.client_message(),
        },
    };
    (result, warnings)
}
pub(crate) async fn find_pages(
    requests: &[FindRequest],
    pages: Vec<PageContent>,
    patterns: Vec<Regex>,
    chunker: TokenChunker,
    config: FindConfig,
    chunk_tokens: usize,
    mut warnings: Vec<String>,
) -> Result<FindResponse> {
    let tasks = requests.iter().zip(pages).zip(patterns).enumerate().map(
        |(request_index, ((request, page), pattern))| {
            let page_chunker = chunker.clone();
            let find_config = config.clone();
            let requested_snippet_tokens = request.snippet_tokens;
            tokio::task::spawn_blocking(move || {
                let mut page_warnings = Vec::new();
                let snippet_tokens = snippet_tokens_for_request(
                    requested_snippet_tokens,
                    request_index,
                    &mut page_warnings,
                    chunk_tokens,
                    find_config.default_snippet_tokens,
                );
                let found =
                    find_in_page(&page, &pattern, snippet_tokens, &page_chunker, &find_config)?;
                Ok::<(FindPage, Vec<String>), AppError>((found, page_warnings))
            })
        },
    );
    let joined = try_join_all(tasks)
        .await
        .map_err(|error| AppError::internal(format!("page processing task failed: {error}")))?;
    let mut found = Vec::with_capacity(joined.len());
    for result in joined {
        let (page, page_warnings) = result?;
        found.push(page);
        warnings.extend(page_warnings);
    }
    Ok(FindResponse {
        pages: found,
        warning: (!warnings.is_empty()).then_some(warnings),
    })
}
fn snippet_tokens_for_request(
    requested: Option<usize>,
    request_index: usize,
    warnings: &mut Vec<String>,
    chunk_tokens: usize,
    default_snippet_tokens: usize,
) -> usize {
    let Some(value) = requested else {
        return default_snippet_tokens;
    };
    if value <= chunk_tokens {
        return value;
    }
    warnings . push (format ! ("\"requests[{request_index}].snippet_tokens\" exceeds chunk_tokens ({chunk_tokens}); using {chunk_tokens}")) ;
    chunk_tokens
}
