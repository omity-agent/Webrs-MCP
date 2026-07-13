use super::extract_markdowns;
use crate::{Result, error::AppError};
#[test]
#[expect(
    clippy::panic_in_result_fn,
    reason = "The test uses assertions while Result keeps setup failures readable."
)]
fn response_extracts_requested_result_text() -> Result<()> {
    let body = br##"{
        "results": [
            {
                "url": "https://example.com",
                "text": "# Example"
            }
        ],
        "errors": []
    }"##;
    let urls = vec!["https://example.com".to_owned()];
    let markdowns = extract_markdowns(&urls, body)?;
    let markdown = markdowns
        .first()
        .ok_or_else(|| AppError::internal("expected one TinyFish result"))?;
    assert_eq!(markdowns.len(), 1);
    assert_eq!(markdown.as_deref().map_err(Clone::clone)?, "# Example");
    Ok(())
}
#[test]
#[expect(
    clippy::panic_in_result_fn,
    reason = "The test uses assertions while Result keeps setup failures readable."
)]
fn response_extracts_batch_results_in_request_order() -> Result<()> {
    let body = br##"{
        "results": [
            {
                "url": "https://example.com/b",
                "text": "# B"
            },
            {
                "url": "https://example.com/a",
                "text": "# A"
            }
        ],
        "errors": []
    }"##;
    let urls = vec![
        "https://example.com/a".to_owned(),
        "https://example.com/b".to_owned(),
    ];
    let markdowns = extract_markdowns(&urls, body)?;
    let first = markdowns
        .first()
        .ok_or_else(|| AppError::internal("first TinyFish result was missing"))?;
    let second = markdowns
        .get(1)
        .ok_or_else(|| AppError::internal("second TinyFish result was missing"))?;
    assert_eq!(markdowns.len(), 2);
    assert_eq!(first.as_deref().map_err(Clone::clone)?, "# A");
    assert_eq!(second.as_deref().map_err(Clone::clone)?, "# B");
    Ok(())
}
#[test]
#[expect(
    clippy::panic_in_result_fn,
    reason = "The test uses assertions while Result keeps setup failures readable."
)]
fn response_preserves_mixed_results_and_errors_in_request_order() -> Result<()> {
    let body = br##"{
        "results": [
            {
                "url": "https://example.com/b",
                "text": "# B"
            }
        ],
        "errors": [
            {
                "url": "https://example.com/a",
                "error": "bot_blocked",
                "status": null
            }
        ]
    }"##;
    let urls = vec![
        "https://example.com/a".to_owned(),
        "https://example.com/b".to_owned(),
        "https://example.com/missing".to_owned(),
    ];
    let markdowns = extract_markdowns(&urls, body)?;
    let first = markdowns
        .first()
        .ok_or_else(|| AppError::internal("first TinyFish result was missing"))?;
    let second = markdowns
        .get(1)
        .ok_or_else(|| AppError::internal("second TinyFish result was missing"))?;
    let third = markdowns
        .get(2)
        .ok_or_else(|| AppError::internal("third TinyFish result was missing"))?;
    assert_eq!(markdowns.len(), 3);
    let first_error = first.as_ref().unwrap_err().client_message();
    assert_eq!(
        first_error,
        "TinyFish could not fetch https://example.com/a: bot_blocked."
    );
    assert_eq!(second.as_deref().map_err(Clone::clone)?, "# B");
    let third_error = third.as_ref().unwrap_err().client_message();
    assert_eq!(
        third_error,
        "TinyFish returned no content for the requested URL: https://example.com/missing."
    );
    Ok(())
}
