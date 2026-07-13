use super::merge_reader_pages;
use crate::{
    Result,
    error::AppError,
    page::fetcher::{PageContent, PageSource},
};
fn direct_page(url: &str) -> PageContent {
    PageContent {
        url: url.to_owned(),
        source: PageSource::Direct,
        markdown: "direct".to_owned(),
    }
}
#[test]
#[expect(
    clippy::panic_in_result_fn,
    reason = "The test uses assertions while Result keeps setup failures readable."
)]
fn merge_preserves_direct_success_and_reader_failure() -> Result<()> {
    let direct_pages = vec![
        Ok(Some(direct_page("https://example.com/a"))),
        Ok(None),
        Ok(None),
    ];
    let missing_urls = vec![
        "https://example.com/b".to_owned(),
        "https://example.com/c".to_owned(),
    ];
    let reader_results = Ok(vec![
        Err(AppError::client("reader failed")),
        Ok("reader content".to_owned()),
    ]);
    let merged = merge_reader_pages(direct_pages, missing_urls, reader_results);
    assert_eq!(merged.len(), 3);
    let first = merged
        .first()
        .and_then(|result| result.as_ref().ok())
        .ok_or_else(|| AppError::internal("first merged page was not successful"))?;
    assert_eq!(first.url, "https://example.com/a");
    assert_eq!(
        merged
            .get(1)
            .and_then(|result| result.as_ref().err())
            .map(AppError::client_message),
        Some("reader failed".to_owned())
    );
    let third = merged
        .get(2)
        .and_then(|result| result.as_ref().ok())
        .ok_or_else(|| AppError::internal("third merged page was not successful"))?;
    assert_eq!(third.markdown, "reader content");
    Ok(())
}
