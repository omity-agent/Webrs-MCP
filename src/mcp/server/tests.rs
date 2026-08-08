use super::{initialize_error, list_tools_result, tool_failure};
use crate::{config, error::AppError, mcp::schemas};
use rmcp::model::{CacheScope, ErrorCode};
#[test]
fn initialize_reports_the_modern_lifecycle() {
    let error = initialize_error();
    assert_eq!(error.code, ErrorCode::METHOD_NOT_FOUND);
    assert!(error.message.contains("2026-07-28"));
    assert!(error.message.contains("server/discover"));
}
#[test]
fn tools_list_has_required_cache_fields() {
    let config = config::load_embedded().unwrap_or_else(|error| panic!("{error}"));
    let tools = schemas::tools().unwrap_or_else(|error| panic!("{error}"));
    let result = list_tools_result(&config, tools);
    assert_eq!(result.ttl_ms, Some(3_600_000));
    assert_eq!(result.cache_scope, Some(CacheScope::Public));
}
#[test]
fn recoverable_tool_failures_are_tool_results() {
    for failure in [
        AppError::client("invalid tool input"),
        AppError::Upstream("upstream unavailable".to_owned()),
    ] {
        let result = tool_failure(failure).unwrap_or_else(|mcp_error| panic!("{mcp_error}"));
        assert_eq!(result.is_error, Some(true));
    }
}
#[test]
fn internal_tool_failures_remain_protocol_errors() {
    let failure = AppError::internal("serialization failed");
    let Err(mcp_error) = tool_failure(failure) else {
        panic!("internal failures must not become model-actionable tool errors");
    };
    assert_eq!(mcp_error.code, ErrorCode::INTERNAL_ERROR);
}
