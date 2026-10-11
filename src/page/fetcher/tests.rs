use super::targets::{direct_fetch_targets, replace_path_suffix};
use crate::{Result, config, error::AppError};
#[test]
#[expect(
    clippy::panic_in_result_fn,
    reason = "The test uses assertions while Result keeps setup failures readable."
)]
fn markdown_fallback_targets_require_markdown_content_type() -> Result<()> {
    let config = config::load_embedded()?;
    let targets = direct_fetch_targets(
        "https://support.apple.com/en-us/121115",
        &config.direct_fetch,
    );
    assert_eq!(targets.len(), 3);
    let markdown_target = targets
        .get(1)
        .ok_or_else(|| AppError::internal("markdown target missing"))?;
    assert!(
        targets
            .iter()
            .all(|target| { target.response_format == crate::direct::ResponseFormat::Markdown })
    );
    assert_eq!(
        markdown_target.request_url,
        "https://support.apple.com/en-us/121115.md"
    );
    assert_eq!(
        targets
            .get(2)
            .ok_or_else(|| AppError::internal("index markdown target missing"))?
            .request_url,
        "https://support.apple.com/en-us/121115/index.md"
    );
    Ok(())
}
#[test]
#[expect(
    clippy::panic_in_result_fn,
    reason = "The test uses assertions while Result keeps setup failures readable."
)]
fn markdown_fallback_replaces_existing_extension_and_shares_probe() -> Result<()> {
    let config = config::load_embedded()?;
    let targets = direct_fetch_targets("https://example.com/foo.html?x=1", &config.direct_fetch);
    let request_urls = targets
        .iter()
        .map(|target| target.request_url.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        request_urls,
        [
            "https://example.com/foo.html?x=1",
            "https://example.com/foo/index.md?x=1",
            "https://example.com/foo.md?x=1",
        ]
    );
    let probe_urls = targets
        .iter()
        .skip(1)
        .map(|target| target.similarity_probe_url.as_deref())
        .collect::<Vec<_>>();
    let first_probe = probe_urls
        .first()
        .ok_or_else(|| AppError::internal("probe URL missing"))?;
    assert!(probe_urls.iter().all(|probe| probe == first_probe));
    Ok(())
}
#[test]
fn markdown_suffix_preserves_query() {
    assert_eq!(
        replace_path_suffix("https://example.com/path?x=1", ".md"),
        "https://example.com/path.md?x=1"
    );
}
