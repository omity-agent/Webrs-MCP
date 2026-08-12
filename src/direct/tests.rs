use crate::{
    Result, config,
    direct::{
        DirectFetchTarget, ResponseFormat, content::extract_content,
        mediawiki::extract_mediawiki_content, resolve_direct_fetch_target,
        stack_overflow::format_stack_overflow_question_json,
    },
};
use reqwest::header::{CONTENT_TYPE, HeaderMap, HeaderValue};
#[test]
#[expect(
    clippy::panic_in_result_fn,
    reason = "The test uses assertions while Result keeps setup failures readable."
)]
fn github_blob_resolves_to_raw_text_url() -> Result<()> {
    let config = config::load_embedded()?;
    let target = resolve_direct_fetch_target(
        "https://github.com/modelcontextprotocol/python-sdk/blob/main/README.md",
        &config.direct_fetch,
    )
    .ok_or_else(|| crate::error::AppError::internal("github target was not resolved"))?;
    assert_eq!(
        target.request_url,
        "https://raw.githubusercontent.com/modelcontextprotocol/python-sdk/main/README.md"
    );
    assert_eq!(target.response_format, ResponseFormat::Text);
    Ok(())
}
#[test]
#[expect(
    clippy::panic_in_result_fn,
    reason = "The test uses assertions while Result keeps setup failures readable."
)]
fn scoped_npm_package_uses_registry_json() -> Result<()> {
    let config = config::load_embedded()?;
    let target = resolve_direct_fetch_target(
        "https://www.npmjs.com/package/@types/node",
        &config.direct_fetch,
    )
    .ok_or_else(|| crate::error::AppError::internal("npm target was not resolved"))?;
    assert_eq!(
        target.request_url,
        "https://registry.npmjs.org/@types%2Fnode"
    );
    assert_eq!(
        target.json_fields_first,
        ["name", "description", "readme", "repository"]
    );
    assert_eq!(target.json_fields_last, ["versions"]);
    Ok(())
}
#[test]
#[expect(
    clippy::panic_in_result_fn,
    reason = "The test uses assertions while Result keeps setup failures readable."
)]
fn npm_registry_json_starts_with_selected_package_fields() -> Result<()> {
    let config = config::load_embedded()?;
    let target = resolve_direct_fetch_target(
        "https://www.npmjs.com/package/example",
        &config.direct_fetch,
    )
    .ok_or_else(|| crate::error::AppError::internal("npm target was not resolved"))?;
    let body = br##"{
        "_id": "example",
        "repository": {"type": "git", "url": "https://example.com/repository.git"},
        "readme": "# Example",
        "description": "Example package",
        "name": "example",
        "dist-tags": {"latest": "1.0.0"},
        "versions": {"1.0.0": {"name": "example"}}
    }"##;
    let content = extract_content(&target, 200, &HeaderMap::new(), body, &config.direct_fetch)?;
    let expected_prefix = concat!(
        "{\n  \"name\": \"example\",\n",
        "  \"description\": \"Example package\",\n",
        "  \"readme\": \"# Example\",\n",
        "  \"repository\": {"
    );
    assert!(
        content.starts_with(expected_prefix),
        "unexpected npm registry JSON field order:\n{content}"
    );
    assert!(content.contains("\"_id\": \"example\""));
    assert!(content.ends_with("  }\n}"));
    Ok(())
}
#[test]
#[expect(
    clippy::panic_in_result_fn,
    reason = "The test uses assertions while Result keeps setup failures readable."
)]
fn stack_overflow_question_resolves_to_api_json() -> Result<()> {
    let config = config::load_embedded()?;
    let target = resolve_direct_fetch_target(
        "https://stackoverflow.com/questions/11828270/how-do-i-exit-vim",
        &config.direct_fetch,
    )
    .ok_or_else(|| crate::error::AppError::internal("Stack Overflow target was not resolved"))?;
    assert_eq!(
        target.request_url,
        "https://api.stackexchange.com/2.3/questions/11828270?order=desc&sort=votes&site=stackoverflow&page=1&pagesize=100&filter=W-vZ8WEHVi3D2JhQe1m8l90EjOxo6eCsb6b_6yfX0_p"
    );
    assert_eq!(
        target.response_format,
        ResponseFormat::StackOverflowQuestionJson
    );
    Ok(())
}
#[test]
#[expect(
    clippy::panic_in_result_fn,
    reason = "The test uses assertions while Result keeps setup failures readable."
)]
fn stack_overflow_json_starts_with_question_then_answers_without_comments() -> Result<()> {
    let payload = br#"{
            "items": [
                {
                    "question_id": 1,
                    "title": "How?",
                    "comment_count": 7,
                    "comments": [{"comment_id": 9}],
                    "answers": [
                        {
                            "answer_id": 2,
                            "is_accepted": true,
                            "comment_count": 3,
                            "comments": [{"comment_id": 4}],
                            "body_markdown": "Answer"
                        }
                    ],
                    "body_markdown": "Question"
                }
            ],
            "has_more": false
        }"#;
    let formatted = format_stack_overflow_question_json(payload)?;
    assert!(formatted.starts_with("{\n  \"question\""));
    assert!(formatted.contains("\"answers\""));
    assert!(!formatted.contains("comment"));
    Ok(())
}
#[test]
fn stack_overflow_rejects_incomplete_answer_list() {
    let result = format_stack_overflow_question_json(br#"{"items":[],"has_more":true}"#);
    match result {
        Ok(content) => panic!("incomplete answer list should be rejected: {content}"),
        Err(error) => assert_eq!(
            error.client_message(),
            "Stack Exchange API returned more than 100 answers; direct fetch cannot return a complete answer list."
        ),
    }
}
#[test]
fn mediawiki_reports_api_error_code() {
    let result = extract_mediawiki_content(br#"{"error":{"code":"badtitle"}}"#);
    match result {
        Ok(content) => panic!("MediaWiki API error should be rejected: {content}"),
        Err(error) => assert_eq!(
            error.client_message(),
            "MediaWiki API rejected the page request (badtitle)."
        ),
    }
}
#[test]
#[expect(
    clippy::panic_in_result_fn,
    reason = "The test uses assertions while Result keeps setup failures readable."
)]
fn json_crlf_line_endings_are_normalized_to_lf() -> Result<()> {
    let config = config::load_embedded()?;
    let target = DirectFetchTarget::mediawiki(
        "https://en.wikipedia.org/wiki/Rust",
        "https://en.wikipedia.org/w/api.php".to_owned(),
    );
    let body = br#"{"query":{"pages":[{"revisions":[{"slots":{"main":{"content":"line1\r\nline2\r\n"}}}]}]}}"# ;
    let content = extract_content(&target, 200, &HeaderMap::new(), body, &config.direct_fetch)?;
    assert!(!content.contains('\r'));
    assert_eq!(content, "line1\nline2\n");
    Ok(())
}
#[test]
#[expect(
    clippy::panic_in_result_fn,
    reason = "The test uses assertions while Result keeps content extraction failures readable."
)]
fn required_content_type_uses_mime_essence() -> Result<()> {
    let config = config::load_embedded()?;
    let target = DirectFetchTarget::markdown_accept("https://example.com/page");
    let mut headers = HeaderMap::new();
    headers.insert(
        CONTENT_TYPE,
        HeaderValue::from_static("Text/Markdown; charset=utf-8"),
    );
    assert_eq!(
        extract_content(&target, 200, &headers, b"# Page", &config.direct_fetch)?,
        "# Page"
    );
    Ok(())
}
