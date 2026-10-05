use super::harness::renamed_config;
use web_rs::{
    cli::{
        Cli,
        oneshot::{OutputFormat, SingleRequest},
    },
    config,
};
#[test]
fn configured_subcommands_parse_one_typed_request() {
    let settings = renamed_config();
    let options = Cli::try_parse_configured_from(
        &settings,
        [
            "web-rs",
            "lookup",
            "--q",
            "Rust tools",
            "--recency",
            "7",
            "--domains",
            "example.org,example.com",
            "--domains",
            "docs.rs",
            "--category",
            "research paper",
            "--exa-api-key",
            "secret",
        ],
    )
    .unwrap()
    .runtime_options()
    .unwrap();
    assert!(options.credentials.search.is_some());
    let invocation = options.invocation.unwrap();
    assert_eq!(invocation.format, OutputFormat::Xml);
    let SingleRequest::Search(request) = invocation.request else {
        panic!("expected one search request");
    };
    assert_eq!(request.q, "Rust tools");
    assert_eq!(request.recency, Some(7));
    assert_eq!(
        request.domains.unwrap(),
        ["example.org", "example.com", "docs.rs"]
    );
    assert_eq!(request.category.unwrap().as_str(), "research paper");
}
#[test]
fn page_commands_parse_defaults_and_regex_parameters() {
    let settings = renamed_config();
    let open = Cli::try_parse_configured_from(
        &settings,
        ["web-rs", "browse", "--url", "https://example.org"],
    )
    .unwrap()
    .runtime_options()
    .unwrap()
    .invocation
    .unwrap();
    let SingleRequest::Open(open_request) = open.request else {
        panic!("expected one open request");
    };
    assert_eq!(open_request.chunk, 0);
    let find = Cli::try_parse_configured_from(
        &settings,
        [
            "web-rs",
            "scan",
            "--url",
            "https://example.org",
            "--pattern",
            "-needle|other",
            "--snippet-tokens",
            "30",
            "--format",
            "json",
        ],
    )
    .unwrap()
    .runtime_options()
    .unwrap()
    .invocation
    .unwrap();
    assert_eq!(find.format, OutputFormat::Json);
    let SingleRequest::Find(find_request) = find.request else {
        panic!("expected one find request");
    };
    assert_eq!(find_request.pattern, "-needle|other");
    assert_eq!(find_request.snippet_tokens, Some(30));
}
#[test]
fn invalid_or_repeated_parameters_are_explicit_errors() {
    let settings = config::load_embedded().unwrap();
    for arguments in [
        vec!["web-rs", "search_query", "--q", "first", "--q", "second"],
        vec![
            "web-rs",
            "open",
            "--url",
            "https://example.org",
            "--chunk",
            "-1",
        ],
        vec![
            "web-rs",
            "find",
            "--url",
            "https://example.org",
            "--pattern",
            "x",
            "--snippet-tokens",
            "0",
        ],
        vec!["web-rs", "search_query", "--q", "x", "--recency", "0"],
        vec!["web-rs", "search_query", "--q", "x", "--format", "yaml"],
        vec![
            "web-rs",
            "--transport",
            "stdio",
            "open",
            "--url",
            "https://example.org",
        ],
        vec![
            "web-rs",
            "--mode",
            "filesystem",
            "open",
            "--url",
            "https://example.org",
        ],
    ] {
        let error = Cli::try_parse_configured_from(&settings, arguments).unwrap_err();
        assert_ne!(error.kind(), clap::error::ErrorKind::DisplayHelp);
    }
}
#[test]
fn configuration_bootstrap_leaves_tool_parameters_for_full_parsing() {
    let path = Cli::config_path_from([
        "web-rs",
        "--exa-api-key",
        "secret",
        "--config",
        "settings.yaml",
        "lookup",
        "--q",
        "one",
        "--format",
        "json",
    ])
    .unwrap()
    .unwrap();
    assert_eq!(path, std::path::PathBuf::from("settings.yaml"));
    assert!(
        Cli::config_path_from(["web-rs", "--help"])
            .unwrap()
            .is_none()
    );
}
