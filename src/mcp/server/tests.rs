use super::{discover_result, list_tools_result, tool_failure};
use crate::{config, error::AppError, mcp::schemas};
use rmcp::model::{CacheScope, ErrorCode, ProtocolVersion};
#[test]
fn server_discovery_advertises_the_modern_contract() {
    let config = config::load_embedded().unwrap_or_else(|error| panic!("{error}"));
    let result = discover_result(&config);
    assert_eq!(
        result.supported_versions,
        vec![ProtocolVersion::V_2026_07_28]
    );
    let server_info = result
        .server_info()
        .unwrap_or_else(|| panic!("server discovery has no server identity"));
    assert_eq!(server_info.name, "web");
    assert_eq!(server_info.version, crate::VERSION);
    assert!(result.capabilities.tools.is_some());
    assert_eq!(
        result.instructions.as_deref(),
        Some("Browsing the Internet")
    );
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
