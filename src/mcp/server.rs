use crate::{
    VERSION,
    config::AppConfig,
    error::AppError,
    mcp::{
        schemas,
        tools::{ToolOutput, ToolService},
    },
};
use axum::http::HeaderMap;
use rmcp::{
    ErrorData as McpError, ServerHandler,
    model::{
        CallToolRequestParams, CallToolResult, ContentBlock, Implementation, JsonObject,
        ListToolsResult, PaginatedRequestParams, ProtocolVersion, ServerCapabilities, ServerInfo,
        Tool,
    },
    service::{MaybeSendFuture, RequestContext, RoleServer},
};
use sonic_rs::Value;
#[expect(
    clippy::missing_trait_methods,
    reason = "RMCP ServerHandler defaults are intentionally used for unsupported protocol hooks."
)]
impl ServerHandler for ToolService {
    #[inline]
    fn get_info(&self) -> ServerInfo {
        server_info(self.config())
    }
    #[inline]
    #[expect(
        clippy::manual_async_fn,
        reason = "The trait requires return-position futures with MaybeSendFuture bounds."
    )]
    fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<ListToolsResult, McpError>> + MaybeSendFuture + '_ {
        async move {
            schemas::tools()
                .map(ListToolsResult::with_all_items)
                .map_err(to_mcp_error)
        }
    }
    #[inline]
    fn get_tool(&self, name: &str) -> Option<Tool> {
        schemas::tool_by_name(name).ok().flatten()
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
    ) -> impl Future<Output = Result<CallToolResult, McpError>> + MaybeSendFuture + '_ {
        async move {
            let empty_headers = HeaderMap::new();
            let headers = request_headers(&context).unwrap_or(&empty_headers);
            let arguments = sonic_arguments(request.arguments)?;
            let output = self
                .call(request.name.as_ref(), arguments, headers)
                .await
                .map_err(to_mcp_error)?;
            tool_result(&output)
        }
    }
}
fn server_info(config: &AppConfig) -> ServerInfo {
    let capabilities = ServerCapabilities::builder().enable_tools().build();
    ServerInfo::new(capabilities)
        .with_server_info(Implementation::new(config.server.name.clone(), VERSION))
        .with_protocol_version(protocol_version(&config.server.protocol_version))
        .with_instructions(config.server.instructions.clone())
}
fn protocol_version(value: &str) -> ProtocolVersion {
    let json_value = rmcp::serde_json::Value::String(value.to_owned());
    rmcp::serde_json::from_value(json_value).unwrap_or_else(|_| ProtocolVersion::default())
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
    let structured = output.structured().map_err(to_mcp_error)?;
    let bytes = sonic_rs::to_vec(&structured).map_err(|error| {
        McpError::internal_error(format!("failed to encode result: {error}"), None)
    })?;
    let json = rmcp::serde_json::from_slice(&bytes).map_err(|error| {
        McpError::internal_error(format!("failed to bridge result: {error}"), None)
    })?;
    let mut result = CallToolResult::success(vec![ContentBlock::text(output.standard_text())]);
    result.structured_content = Some(json);
    Ok(result)
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
