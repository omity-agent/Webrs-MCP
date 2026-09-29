use crate::{
    VERSION,
    config::AppConfig,
    error::AppError,
    mcp::{
        schemas,
        tools::{ToolOutput, ToolService},
    },
};
use alloc::borrow::Cow;
use axum::http::HeaderMap;
use rmcp::{
    ErrorData as McpError, ServerHandler,
    model::{
        CallToolRequestParams, CallToolResponse, CallToolResult, ContentBlock, DiscoverResult,
        Implementation, JsonObject, ListToolsResult, PaginatedRequestParams, ProtocolVersion,
        ServerCapabilities, ServerConfig, Tool,
    },
    service::{MaybeSendFuture, RequestContext, RoleServer},
};
use sonic_rs::Value;
const SUPPORTED_PROTOCOL_VERSIONS: &[ProtocolVersion] = &[ProtocolVersion::V_2026_07_28];
#[cfg(test)]
mod tests;
#[expect(
    clippy::missing_trait_methods,
    reason = "RMCP defaults are intentionally used for protocol hooks this tools-only server does not advertise."
)]
impl ServerHandler for ToolService {
    #[inline]
    fn get_info(&self) -> ServerConfig {
        server_info(self.config())
    }
    #[inline]
    fn supported_protocol_versions(&self) -> Cow<'static, [ProtocolVersion]> {
        Cow::Borrowed(SUPPORTED_PROTOCOL_VERSIONS)
    }
    #[inline]
    fn discover(
        &self,
        _context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<DiscoverResult, McpError>> + MaybeSendFuture + '_ {
        core::future::ready(Ok(discover_result(self.config())))
    }
    #[inline]
    fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<ListToolsResult, McpError>> + MaybeSendFuture + '_ {
        let result = schemas::tools(self.mode())
            .map(|tools| list_tools_result(self.config(), tools))
            .map_err(to_mcp_error);
        core::future::ready(result)
    }
    #[inline]
    fn get_tool(&self, name: &str) -> Option<Tool> {
        schemas::tool_by_name(self.mode(), name).ok().flatten()
    }
    #[inline]
    #[expect(
        clippy::manual_async_fn,
        reason = "The trait requires return-position futures with MaybeSendFuture bounds."
    )]
    fn call_tool(
        &self,
        request: CallToolRequestParams,
        context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<CallToolResponse, McpError>> + MaybeSendFuture + '_ {
        async move {
            let tool_name = request.name.as_ref();
            if schemas::tool_by_name(self.mode(), tool_name)
                .map_err(to_mcp_error)?
                .is_none()
            {
                return Err(McpError::invalid_params(
                    format!("Unknown tool: {tool_name}"),
                    None,
                ));
            }
            let empty_headers = HeaderMap::new();
            let headers = request_headers(&context).unwrap_or(&empty_headers);
            let arguments = sonic_arguments(request.arguments)?;
            match self.call(tool_name, arguments, headers).await {
                Ok(output) => tool_result(&output).map(Into::into),
                Err(error) => tool_failure(error).map(Into::into),
            }
        }
    }
}
fn list_tools_result(config: &AppConfig, tools: Vec<Tool>) -> ListToolsResult {
    let cache = &config.protocol.tools_list_cache;
    ListToolsResult::with_all_items(tools)
        .with_ttl_ms(cache.ttl_ms)
        .with_cache_scope(cache.scope)
}
fn discover_result(config: &AppConfig) -> DiscoverResult {
    DiscoverResult::from_server_info(SUPPORTED_PROTOCOL_VERSIONS.to_vec(), server_info(config))
}
fn server_info(config: &AppConfig) -> ServerConfig {
    let capabilities = ServerCapabilities::builder().enable_tools().build();
    ServerConfig::new(capabilities)
        .with_server_info(Implementation::new(config.server.name.clone(), VERSION))
        .with_protocol_version(ProtocolVersion::V_2026_07_28)
        .with_instructions(config.server.instructions.clone())
}
fn request_headers(context: &RequestContext<RoleServer>) -> Option<&HeaderMap> {
    context
        .extensions
        .get::<http::request::Parts>()
        .map(|parts| &parts.headers)
}
fn sonic_arguments(arguments: Option<JsonObject>) -> Result<Option<Value>, McpError> {
    let Some(raw_arguments) = arguments else {
        return Ok(None);
    };
    sonic_rs::to_value(&rmcp::serde_json::Value::Object(raw_arguments))
        .map(Some)
        .map_err(|error| {
            McpError::internal_error(format!("failed to read arguments: {error}"), None)
        })
}
pub(crate) fn tool_result(output: &ToolOutput) -> Result<CallToolResult, McpError> {
    let mut result = CallToolResult::success(vec![ContentBlock::text(output.standard_text())]);
    result.structured_content = Some(output.structured().map_err(to_mcp_error)?);
    Ok(result)
}
fn tool_failure(error: AppError) -> Result<CallToolResult, McpError> {
    match error {
        AppError::Client(message) | AppError::Upstream(message) => {
            Ok(CallToolResult::error(vec![ContentBlock::text(message)]))
        }
        failure @ (AppError::Config(_) | AppError::Internal(_)) => Err(to_mcp_error(failure)),
    }
}
fn to_mcp_error(error: AppError) -> McpError {
    match error {
        AppError::Client(message) => McpError::invalid_params(message, None),
        AppError::Config(message) | AppError::Upstream(message) => {
            McpError::internal_error(message, None)
        }
        AppError::Internal(_) => McpError::internal_error(
            "Unexpected server error. Retry the request or contact the service operator.",
            None,
        ),
    }
}
