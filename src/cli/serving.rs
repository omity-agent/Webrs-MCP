use super::{Transport, WorkMode};
use crate::{Result, error::AppError};
use clap::{Args, Command};
#[derive(Clone, Copy, Debug, Args)]
pub struct McpOptions {
    #[arg(long, value_enum, default_value = "http")]
    pub transport: Transport,
    #[arg(long, value_enum, default_value_t)]
    pub mode: WorkMode,
    #[arg(long, help = "Return full pages without chunk fields")]
    pub no_chunking: bool,
}
impl McpOptions {
    pub(super) fn validate(self) -> Result<()> {
        if self.mode == WorkMode::Filesystem && self.transport != Transport::Stdio {
            return Err(AppError::config(
                "filesystem mode can only be used with --transport stdio.",
            ));
        }
        Ok(())
    }
}
pub(super) fn definition() -> Command {
    McpOptions::augment_args(
        Command::new("mcp")
            .about("Run the MCP server")
            .after_help("HTTP MCP reads API keys from request headers. Stdio MCP reads API keys from environment variables."),
    )
}
