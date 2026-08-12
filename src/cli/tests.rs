use super::{Cli, Transport, WorkMode};
use clap::Parser as _;
#[test]
fn exa_api_key_configures_credentials() {
    let cli = Cli::try_parse_from(["web-rs", "--exa-api-key", "exa"])
        .unwrap_or_else(|error| panic!("{error}"));
    let options = cli
        .runtime_options()
        .unwrap_or_else(|error| panic!("{error}"));
    assert_eq!(options.credentials.exa_api_key.as_deref(), Some("exa"));
}
#[test]
fn http_transport_is_default() {
    let cli = Cli::try_parse_from(["web-rs"]).unwrap_or_else(|error| panic!("{error}"));
    let options = cli
        .runtime_options()
        .unwrap_or_else(|error| panic!("{error}"));
    assert_eq!(options.transport, Transport::Http);
    assert_eq!(options.mode, WorkMode::Response);
}
#[test]
fn filesystem_mode_requires_stdio() {
    let cli = Cli::try_parse_from(["web-rs", "--mode", "filesystem"])
        .unwrap_or_else(|error| panic!("{error}"));
    let error = cli.runtime_options().unwrap_err().client_message();
    assert!(error.contains("--transport stdio"));
}
#[test]
fn filesystem_mode_accepts_stdio() {
    let cli = Cli::try_parse_from(["web-rs", "--transport", "stdio", "--mode", "filesystem"])
        .unwrap_or_else(|error| panic!("{error}"));
    let options = cli
        .runtime_options()
        .unwrap_or_else(|error| panic!("{error}"));
    assert_eq!(options.mode, WorkMode::Filesystem);
}
#[test]
fn transport_can_select_stdio() {
    let cli = Cli::try_parse_from(["web-rs", "--transport", "stdio"])
        .unwrap_or_else(|error| panic!("{error}"));
    let options = cli
        .runtime_options()
        .unwrap_or_else(|error| panic!("{error}"));
    assert_eq!(options.transport, Transport::Stdio);
}
#[test]
fn rejects_multiple_reader_api_keys() {
    let cli = Cli::try_parse_from([
        "web-rs",
        "--jina-api-key",
        "jina",
        "--tinyfish-api-key",
        "tinyfish",
    ])
    .unwrap_or_else(|error| panic!("{error}"));
    let error = cli.runtime_options().unwrap_err().client_message();
    assert!(error.contains("at most one"));
}
