use crate::{
    listener::LocalServer,
    process::{invoke, renamed_settings, settings_file, stdout},
};
use axum::{Router, routing::get};
use rmcp::serde_json::Value;
#[tokio::test]
async fn invalid_cli_input_exits_without_success_output_or_network_io() {
    let server =
        LocalServer::start(Router::new().route("/page", get(|| async { "unexpected" }))).await;
    let file = settings_file(&renamed_settings());
    let url = format!("{}/page", server.address);
    for (arguments, expected) in [
        (vec!["lookup"], "QUERY"),
        (vec!["browse"], "URL"),
        (vec!["lookup", ""], "value"),
        (vec!["browse", ""], "value"),
        (vec!["lookup", "one", "two"], "two"),
        (vec!["lookup", "test", "--recency", "0"], "--recency"),
        (vec!["browse", &url, "--mode", "filesystem"], "--mode"),
        (
            vec!["lookup", "test", "--transport", "stdio"],
            "--transport",
        ),
        (vec!["mcp", "--mode", "filesystem"], "stdio"),
    ] {
        let output = invoke(file.path(), &arguments).await;
        assert!(!output.status.success(), "{arguments:?}");
        assert!(
            output.stdout.is_empty(),
            "unexpected success output: {}",
            stdout(&output)
        );
        let diagnostic = String::from_utf8_lossy(&output.stderr);
        assert!(diagnostic.contains(expected), "{arguments:?}: {diagnostic}");
    }
    assert!(server.visits.lock().await.is_empty());
}
#[tokio::test]
async fn root_requires_a_command_and_help_exposes_only_mcp_search_and_open() {
    let file = settings_file(&renamed_settings());
    let executable = std::path::Path::new(env!("CARGO_BIN_EXE_web"))
        .file_name()
        .unwrap()
        .to_str()
        .unwrap();
    for arguments in [vec![], vec!["--help"]] {
        let output = invoke(file.path(), &arguments).await;
        assert_eq!(output.status.success(), !arguments.is_empty());
        let text = format!(
            "{}{}",
            stdout(&output),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(text.contains(&format!("Usage: {executable} ")), "{text}");
        if arguments.is_empty() {
            assert!(text.contains("requires a subcommand"), "{text}");
            continue;
        }
        for name in ["mcp", "lookup", "browse"] {
            assert!(
                text.lines()
                    .any(|line| line.trim_start().starts_with(&format!("{name} "))),
                "{text}"
            );
        }
        assert!(
            !text
                .lines()
                .any(|line| line.trim_start().starts_with("scan ")),
            "{text}"
        );
        for variable in [
            "EXA_API_KEY",
            "OCTEN_API_KEY",
            "JINA_API_KEY",
            "TINYFISH_API_KEY",
        ] {
            assert!(text.contains(variable), "{text}");
        }
    }
    for (name, positional) in [
        ("lookup", "<QUERY>"),
        ("browse", "<URL>"),
        ("mcp", "--transport"),
    ] {
        let output = invoke(file.path(), &[name, "--help"]).await;
        assert!(output.status.success(), "{output:?}");
        assert!(stdout(&output).contains(positional), "{output:?}");
    }
}
#[tokio::test]
async fn configuration_errors_stop_startup_even_when_only_help_is_requested() {
    let directory = tempfile::tempdir().unwrap();
    let missing = invoke(&directory.path().join("missing.yaml"), &["--help"]).await;
    assert!(!missing.status.success() && missing.stdout.is_empty());
    assert_ne!(missing.stderr.as_slice(), b"");
    for name in [
        "",
        "has space",
        "-flag",
        "a/b",
        "help",
        "mcp",
        "工具",
        "lookup",
    ] {
        let mut settings = renamed_settings();
        settings.tools.open = name.to_owned();
        let file = settings_file(&settings);
        let output = invoke(file.path(), &["--help"]).await;
        assert!(
            !output.status.success() && output.stdout.is_empty(),
            "{name}"
        );
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("tools.open"),
            "{name}: {:?}",
            output.stderr
        );
    }
}
#[tokio::test]
async fn invalid_dynamic_highlights_beta_stops_startup() {
    for beta in ["", " \t", "invalid\nheader"] {
        let mut settings = renamed_settings();
        settings.search.exa.dynamic_highlights_beta = beta.to_owned();
        let file = settings_file(&settings);
        let output = invoke(file.path(), &["--help"]).await;
        assert!(!output.status.success() && output.stdout.is_empty());
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("search.exa.dynamic_highlights_beta"),
            "{beta:?}: {:?}",
            output.stderr
        );
    }
}
#[tokio::test]
async fn failed_page_commands_return_formatted_failure_and_nonzero_exit_status() {
    let server = LocalServer::start(Router::new()).await;
    let file = settings_file(&renamed_settings());
    let url = format!("{}/missing", server.address);
    for format in ["xml", "json"] {
        let output = invoke(file.path(), &["browse", &url, "--format", format]).await;
        assert!(!output.status.success() && !output.stderr.is_empty());
        if format == "json" {
            let reply: Value = sonic_rs::from_str(stdout(&output)).unwrap();
            let result = reply.pointer("/results/0").unwrap();
            assert_eq!(result.get("url").unwrap(), &url);
            assert!(
                result
                    .get("error")
                    .unwrap()
                    .as_str()
                    .unwrap()
                    .contains("API key")
            );
            assert!(result.get("page").is_none());
        } else {
            assert!(stdout(&output).contains("<ERROR>\n"));
            assert!(stdout(&output).contains(&url));
        }
    }
}
