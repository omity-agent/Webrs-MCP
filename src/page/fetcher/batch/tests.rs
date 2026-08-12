use super::{UrlBatch, merge_reader_pages};
use crate::{
    Result,
    error::AppError,
    page::fetcher::{PageContent, PageSource},
};
fn direct_page(url: &str) -> PageContent {
    PageContent {
        url: url.to_owned(),
        source: PageSource::Direct,
        markdown: "direct".into(),
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
    assert_eq!(third.markdown.as_ref(), "reader content");
    Ok(())
}
#[test]
fn duplicate_urls_expand_shared_results_in_original_order() {
    let urls = [
        "https://example.com/a".to_owned(),
        "https://example.com/b".to_owned(),
        "https://example.com/a".to_owned(),
    ];
    let batch = UrlBatch::new(&urls);
    assert_eq!(batch.unique.len(), 2);
    let results = [
        Ok(direct_page("https://example.com/a")),
        Err(AppError::client("page fetch failed")),
    ];
    let expanded = batch.expand(&results);
    assert_eq!(expanded.len(), 3);
    assert!(expanded.first().is_some_and(Result::is_ok));
    assert!(expanded.get(1).is_some_and(Result::is_err));
    assert!(expanded.get(2).is_some_and(Result::is_ok));
    let first = expanded.first().and_then(|result| result.as_ref().ok());
    let third = expanded.get(2).and_then(|result| result.as_ref().ok());
    assert!(first.zip(third).is_some_and(|(first_page, third_page)| {
        alloc::sync::Arc::ptr_eq(&first_page.markdown, &third_page.markdown)
    }));
}
