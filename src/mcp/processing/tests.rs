use super::open_pages;
use crate::{
    Result,
    config::ChunkingConfig,
    error::AppError,
    models::{OpenRequest, OpenResult},
    page::{PageContent, TokenChunker, fetcher::PageSource},
};
#[tokio::test]
#[expect(
    clippy::panic_in_result_fn,
    reason = "The test uses assertions while Result keeps setup failures readable."
)]
async fn open_returns_successes_alongside_per_request_errors() -> Result<()> {
    let requests = vec![
        OpenRequest {
            url: "https://example.com/a".to_owned(),
            chunk: 0,
        },
        OpenRequest {
            url: "https://example.com/b".to_owned(),
            chunk: 0,
        },
    ];
    let fetched = vec![
        Ok(PageContent {
            url: "https://example.com/a".to_owned(),
            source: PageSource::Direct,
            markdown: "successful page".to_owned(),
        }),
        Err(AppError::client("page fetch failed")),
    ];
    let chunker = TokenChunker::new(&ChunkingConfig {
        tokenizer: "o200k_base".to_owned(),
        chunk_tokens: 100,
        overlap_ratio: 0.1_f64,
    })?;
    let response = open_pages(&requests, fetched, chunker, Vec::new()).await;
    assert_eq!(response.results.len(), 2);
    let first = response
        .results
        .first()
        .ok_or_else(|| AppError::internal("first open result was missing"))?;
    assert!(
        matches ! (first , OpenResult :: Success { url , page } if url == "https://example.com/a" && page . content == "successful page")
    );
    let second = response
        .results
        .get(1)
        .ok_or_else(|| AppError::internal("second open result was missing"))?;
    assert!(
        matches ! (second , OpenResult :: Failure { url , error } if url == "https://example.com/b" && error == "page fetch failed")
    );
    let encoded = sonic_rs::to_string(&response)
        .map_err(|error| AppError::internal(format!("failed to encode open response: {error}")))?;
    assert!(encoded.contains("\"results\""));
    assert!(encoded.contains("\"page\""));
    assert!(encoded.contains("\"error\":\"page fetch failed\""));
    assert!(!encoded.contains("\"pages\""));
    Ok(())
}
