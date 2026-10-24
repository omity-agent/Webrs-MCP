mod access;
pub mod oneshot;
mod serving;
mod subcommands;
use crate::{config::AppConfig, error::AppError, mcp::tools::ToolCredentials};
use clap::{CommandFactory as _, FromArgMatches as _, Parser, ValueEnum};
use clap_builder::{Arg, ArgAction, Command, Error};
use oneshot::{Invocation, SingleRequest};
use std::{ffi::OsString, path::PathBuf};
pub type McpOptions = serving::McpOptions;
#[derive(Clone, Debug)]
pub struct RuntimeOptions {
    pub command: RuntimeCommand,
    pub credentials: ToolCredentials,
}
#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum RuntimeCommand {
    Mcp(McpOptions),
    Tool(Invocation),
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
#[non_exhaustive]
pub enum Transport {
    Http,
    Stdio,
}
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, ValueEnum)]
#[non_exhaustive]
pub enum WorkMode {
    #[default]
    Response,
    Filesystem,
}
#[derive(Debug, Parser)]
#[command(name = "web")]
pub struct Cli {
    #[arg(
        long,
        value_name = "FILE",
        help = "Load configuration before the tool command"
    )]
    config: Option<PathBuf>,
    #[arg(skip)]
    command: Option<RuntimeCommand>,
}
impl Cli {
    #[inline]
    #[must_use]
    pub fn from_env(config: &AppConfig) -> Self {
        Self::try_parse_configured_from(config, std::env::args_os())
            .unwrap_or_else(|error| error.exit())
    }
    #[inline]
    #[must_use]
    pub fn configured_command(config: &AppConfig) -> Command {
        Self::command()
            .disable_version_flag(true)
            .subcommand_required(true)
            .arg_required_else_help(true)
            .subcommand(serving::definition())
            .subcommands(subcommands::definitions(&config.tools))
            .after_help(format!(
                "API key environment variables (CLI and stdio MCP only):\n  {} or {} (search)\n  {} or {} (remote reader)\nHTTP MCP accepts API keys only through request headers.",
                config.environment.exa,
                config.environment.octen,
                config.environment.jina,
                config.environment.tinyfish,
            ))
    }
    #[inline]
    pub fn config_path_from<I, T>(arguments: I) -> Result<Option<PathBuf>, Error>
    where
        I: IntoIterator<Item = T>,
        T: Into<OsString> + Clone,
    {
        let matches = Self::command()
            .disable_help_flag(true)
            .disable_version_flag(true)
            .disable_help_subcommand(true)
            .allow_external_subcommands(true)
            .arg(
                Arg::new("help")
                    .long("help")
                    .short('h')
                    .action(ArgAction::SetTrue),
            )
            .try_get_matches_from(arguments)?;
        Ok(Self::from_arg_matches(&matches)?.config)
    }
    #[inline]
    pub fn try_parse_configured_from<I, T>(config: &AppConfig, arguments: I) -> Result<Self, Error>
    where
        I: IntoIterator<Item = T>,
        T: Into<OsString> + Clone,
    {
        let mut command = Self::configured_command(config);
        let matches = command.try_get_matches_from_mut(arguments)?;
        let (name, subarguments) = matches.subcommand().ok_or_else(|| {
            command.error(
                clap::error::ErrorKind::MissingSubcommand,
                "A command is required.",
            )
        })?;
        let selected = if name == "mcp" {
            RuntimeCommand::Mcp(McpOptions::from_arg_matches(subarguments)?)
        } else {
            RuntimeCommand::Tool(subcommands::invocation(&config.tools, name, subarguments)?)
        };
        let mut cli = Self::from_arg_matches(&matches)?;
        cli.command = Some(selected);
        Ok(cli)
    }
    #[inline]
    pub fn runtime_options(self, config: &AppConfig) -> crate::Result<RuntimeOptions> {
        let command = self
            .command
            .ok_or_else(|| AppError::config("A command is required."))?;
        let credentials = if matches!(
            command,
            RuntimeCommand::Mcp(McpOptions {
                transport: Transport::Http,
                mode: _
            })
        ) {
            ToolCredentials::default()
        } else {
            access::credentials(&config.environment)?
        };
        if credentials.search.is_none()
            && matches!(
                command,
                RuntimeCommand::Tool(Invocation {
                    request: SingleRequest::Search(_),
                    format: _,
                })
            )
        {
            return Err(AppError::config(format!(
                "Missing search API key: set {} or {}.",
                config.environment.exa, config.environment.octen
            )));
        }
        let options = RuntimeOptions {
            command,
            credentials,
        };
        options.validate()?;
        Ok(options)
    }
}
impl RuntimeOptions {
    #[inline]
    pub fn validate(&self) -> crate::Result<()> {
        if let RuntimeCommand::Mcp(options) = self.command {
            options.validate()?;
        }
        Ok(())
    }
}
