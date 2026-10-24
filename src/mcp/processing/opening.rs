use super::BlockingPool;
use crate::{
    Result,
    error::AppError,
    models::{ChunkSelection, OpenRequest, OpenResponse, OpenResult},
    page::{PageContent, TokenChunker, open_page},
    warnings::Warnings,
};
use futures::{StreamExt as _, stream};
pub(crate) async fn open_pages(
    requests: &[OpenRequest],
    pages: Vec<Result<PageContent>>,
    chunker: TokenChunker,
    pool: &BlockingPool,
    mut warnings: Warnings,
) -> OpenResponse {
    let mut fetched_pages = pages.into_iter();
    let max_concurrent_tasks = chunker.max_concurrent_tasks();
    let tasks = requests.iter().cloned().enumerate().map(
        |(request_index, request): (usize, OpenRequest)| {
            let fetched = fetched_pages
                .next()
                .unwrap_or_else(|| Err(AppError::internal("page fetch result was missing")));
            let page_chunker = chunker.clone();
            let selection = request.selection;
            let result_url = request.url.clone();
            let task_url = request.url;
            async move {
                if selection.is_none() {
                    return process_open_result(
                        task_url,
                        fetched,
                        selection,
                        request_index,
                        &page_chunker,
                    );
                }
                let task = pool.run(move || {
                    Ok(process_open_result(
                        task_url,
                        fetched,
                        selection,
                        request_index,
                        &page_chunker,
                    ))
                });
                task.await.unwrap_or_else(|error| {
                    (
                        OpenResult::Failure {
                            url: result_url,
                            error: error.client_message(),
                        },
                        Warnings::new(),
                    )
                })
            }
        },
    );
    let joined = stream::iter(tasks)
        .buffered(max_concurrent_tasks)
        .collect::<Vec<_>>()
        .await;
    let mut results = Vec::with_capacity(joined.len());
    for (result, page_warnings) in joined {
        results.push(result);
        warnings.append(page_warnings);
    }
    OpenResponse {
        results,
        warning: warnings.into_option(),
    }
}
fn process_open_result(
    url: String,
    fetched: Result<PageContent>,
    selection: Option<ChunkSelection>,
    request_index: usize,
    chunker: &TokenChunker,
) -> (OpenResult, Warnings) {
    let mut warnings = Warnings::new();
    let opened = match fetched {
        Ok(page) => open_page(&page, selection, request_index, chunker, &mut warnings),
        Err(error) => Err(error),
    };
    let result = match opened {
        Ok(page) => OpenResult::Success { url, page },
        Err(error) => OpenResult::Failure {
            url,
            error: error.client_message(),
        },
    };
    (result, warnings)
}
