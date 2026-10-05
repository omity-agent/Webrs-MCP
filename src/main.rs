use anyhow::Context as _;
use tracing_subscriber::{EnvFilter, fmt};
use web_rs::{app, cli::Cli, config};
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config_path =
        Cli::config_path_from(std::env::args_os()).unwrap_or_else(|error| error.exit());
    let app_config = config_path
        .as_deref()
        .map_or_else(config::load_embedded, config::load)
        .context("failed to load configuration")?;
    let options = Cli::from_env(&app_config)
        .runtime_options()
        .context("failed to parse command line")?;
    install_tracing(&app_config.server.log_level)?;
    app::run(app_config, options)
        .await
        .context("web command failed")?;
    Ok(())
}
fn install_tracing(log_level: &str) -> anyhow::Result<()> {
    let directives = match std::env::var("RUST_LOG") {
        Ok(value) => value,
        Err(std::env::VarError::NotPresent) => log_level.to_owned(),
        Err(error @ std::env::VarError::NotUnicode(_)) => return Err(error.into()),
    };
    let filter = EnvFilter::try_new(directives).context("invalid logging filter")?;
    fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .try_init()
        .map_err(|error| anyhow::anyhow!("failed to install tracing subscriber: {error}"))?;
    Ok(())
}
