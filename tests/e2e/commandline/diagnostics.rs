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
        (vec!["lookup", "--q", "test"], "API key"),
        (
            vec![
                "lookup",
                "--q",
                "test",
                "--exa-api-key",
                "exa",
                "--octen-api-key",
                "octen",
            ],
            "API key",
        ),
        (
            vec!["lookup", "--q", "test", "--exa-api-key", " "],
            "API key",
        ),
        (vec!["lookup", "--q", "one", "--q", "two"], "--q"),
        (
            vec![
                "browse",
                "--url",
                &url,
                "--jina-api-key",
                "jina",
                "--tinyfish-api-key",
                "tinyfish",
            ],
            "API key",
        ),
        (vec!["browse", "--url", &url, "--chunk=-1"], "--chunk"),
        (
            vec!["--mode", "filesystem", "browse", "--url", &url],
            "--mode",
        ),
        (
            vec!["scan", "--url", &url, "--pattern", "[", "--format", "json"],
            "pattern",
        ),
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
async fn configuration_errors_stop_startup_even_when_only_help_is_requested() {
    let directory = tempfile::tempdir().unwrap();
    let missing = invoke(&directory.path().join("missing.yaml"), &["--help"]).await;
    assert!(!missing.status.success() && missing.stdout.is_empty());
    assert_ne!(missing.stderr.as_slice(), b"");
    for name in ["", "has space", "-flag", "a/b", "help", "工具", "lookup"] {
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
        let output = invoke(file.path(), &["browse", "--url", &url, "--format", format]).await;
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
