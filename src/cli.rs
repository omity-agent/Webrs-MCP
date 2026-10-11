mod access;
pub mod oneshot;
mod subcommands;
use crate::{config::AppConfig, error::AppError, mcp::tools::ToolCredentials};
use clap::{CommandFactory as _, FromArgMatches as _, Parser, ValueEnum};
use clap_builder::{Arg, ArgAction, Command, Error};
use oneshot::Invocation;
use std::{ffi::OsString, path::PathBuf};
#[derive(Clone, Debug)]
pub struct RuntimeOptions {
    pub transport: Transport,
    pub mode: WorkMode,
    pub credentials: ToolCredentials,
    pub invocation: Option<Invocation>,
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
pub struct Cli {
    #[arg(
        long,
        value_name = "FILE",
        help = "Load configuration before the tool command"
    )]
    config: Option<PathBuf>,
    #[arg(long, value_enum, default_value = "http")]
    transport: Transport,
    #[arg(long, value_enum, default_value_t)]
    mode: WorkMode,
    #[arg(long = "exa-api-key", global = true)]
    exa: Option<String>,
    #[arg(long = "octen-api-key", global = true)]
    octen: Option<String>,
    #[arg(long = "jina-api-key", global = true)]
    jina: Option<String>,
    #[arg(long = "tinyfish-api-key", global = true)]
    tinyfish: Option<String>,
    #[arg(skip)]
    invocation: Option<Invocation>,
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
            .subcommands(subcommands::definitions(&config.tools))
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
        if matches.subcommand().is_some()
            && ["transport", "mode"].into_iter().any(|name| {
                matches.value_source(name) == Some(clap::parser::ValueSource::CommandLine)
            })
        {
            return Err(command.error(
                clap::error::ErrorKind::ArgumentConflict,
                "Tool commands cannot be combined with --transport or --mode.",
            ));
        }
        let mut cli = Self::from_arg_matches(&matches)?;
        cli.invocation = subcommands::invocation(&config.tools, &matches)?;
        Ok(cli)
    }
    #[inline]
    pub fn runtime_options(self) -> crate::Result<RuntimeOptions> {
        let credentials = access::credentials(&self)?;
        let options = RuntimeOptions {
            transport: self.transport,
            mode: self.mode,
            credentials,
            invocation: self.invocation,
        };
        options.validate()?;
        Ok(options)
    }
}
impl RuntimeOptions {
    #[inline]
    pub fn validate(&self) -> crate::Result<()> {
        if self.invocation.is_some()
            && (self.mode != WorkMode::Response || self.transport != Transport::Http)
        {
            return Err(AppError::config(
                "Tool commands cannot be combined with --transport or --mode.",
            ));
        }
        if self.mode == WorkMode::Filesystem && self.transport != Transport::Stdio {
            return Err(AppError::config(
                "filesystem mode can only be used with --transport stdio.",
            ));
        }
        Ok(())
    }
}
