use super::{
    arguments,
    storage::{ID_ALPHABET, ID_LENGTH, Invocation},
};
use crate::{Result, page::fetcher::PageContent, page::fetcher::PageSource};
use alloc::sync::Arc;
use tempfile::tempdir;
#[test]
fn open_arguments_require_http_urls() {
    let raw = sonic_rs :: json ! ({ "output_path" : "output" , "urls" : ["file:///secret"] });
    let error = arguments::open(Some(raw)).unwrap_err().client_message();
    assert!(error.contains("HTTP or HTTPS"));
}
#[tokio::test]
#[expect(
    clippy::panic_in_result_fn,
    reason = "Assertions verify the generated filesystem layout while Result keeps I/O setup readable."
)]
async fn invocation_uses_base36_id_and_copies_cached_page() -> Result<()> {
    let temporary = tempdir()
        .map_err(|error| crate::error::AppError::internal(format!("tempdir failed: {error}")))?;
    let output_path = temporary.path().to_string_lossy().into_owned();
    let first = Invocation::create(&output_path).await?;
    first
        .prepare(core::iter::once(&std::path::PathBuf::from("0")))
        .await?;
    let page = PageContent {
        url: "https://example.com/".to_owned(),
        source: PageSource::Direct,
        markdown: Arc::from("complete content"),
    };
    first
        .write_page(std::path::Path::new("0"), "https://example.com/", &page)
        .await?;
    let first_response = first.response(None);
    let id = std::path::Path::new(&first_response.output_path)
        .file_name()
        .and_then(std::ffi::OsStr::to_str)
        .unwrap_or_else(|| panic!("invocation path has no UTF-8 file name"));
    assert_eq!(id.len(), ID_LENGTH);
    assert!(id.chars().all(|character| ID_ALPHABET.contains(&character)));
    let second = Invocation::create(&output_path).await?;
    assert!(second.is_cached("https://example.com/"));
    second
        .prepare(core::iter::once(&std::path::PathBuf::from("0")))
        .await?;
    second
        .copy_cached(std::path::Path::new("0"), "https://example.com/")
        .await?;
    let destination = std::path::PathBuf::from(second.response(None).output_path).join("0");
    assert_eq!(
        tokio::fs::read_to_string(destination.join("content.txt"))
            .await
            .map_err(|error| crate::error::AppError::internal(format!("read failed: {error}")))?,
        "complete content"
    );
    assert_eq!(
        tokio::fs::read_to_string(destination.join(".url.txt"))
            .await
            .map_err(|error| crate::error::AppError::internal(format!("read failed: {error}")))?,
        "https://example.com/"
    );
    Ok(())
}
