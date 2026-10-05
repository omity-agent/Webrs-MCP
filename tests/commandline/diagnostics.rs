use super::harness::{assert_success, binary, invoke, renamed_config, settings_file, stdout};
use web_rs::config;
#[tokio::test]
async fn configured_help_displays_tool_names_and_output_modes() {
    let file = settings_file(&renamed_config());
    let help = invoke(&file, &["--help"]).await;
    assert_success(&help);
    for name in ["lookup", "browse", "scan"] {
        assert!(stdout(&help).contains(name));
    }
    let tool_help = invoke(&file, &["lookup", "--help"]).await;
    assert_success(&tool_help);
    assert!(stdout(&tool_help).contains("--q"));
    assert!(stdout(&tool_help).contains("--format"));
    assert!(stdout(&tool_help).contains("[default: xml]"));
}
#[tokio::test]
async fn missing_or_conflicting_credentials_do_not_emit_success_output() {
    let file = settings_file(&renamed_config());
    for arguments in [
        vec!["lookup", "--q", "test"],
        vec![
            "lookup",
            "--q",
            "test",
            "--exa-api-key",
            "exa",
            "--octen-api-key",
            "octen",
        ],
        vec!["lookup", "--q", "test", "--exa-api-key", " "],
        vec![
            "browse",
            "--url",
            "https://example.org",
            "--jina-api-key",
            "",
        ],
    ] {
        let output = invoke(&file, &arguments).await;
        assert!(!output.status.success());
        assert_eq!(output.stdout.as_slice(), b"");
        assert!(String::from_utf8_lossy(&output.stderr).contains("API key"));
    }
}
#[test]
fn file_loading_rejects_invalid_or_duplicate_tool_names() {
    for name in ["", "has space", "-flag", "a/b", "help", "工具"] {
        let mut settings = renamed_config();
        settings.tools.open = name.to_owned();
        let file = settings_file(&settings);
        let error = config::load(file.path()).unwrap_err();
        assert!(error.client_message().contains("tools.open"));
    }
    let original = renamed_config();
    for names in [
        config::ToolNames {
            open: original.tools.search_query.clone(),
            ..original.tools.clone()
        },
        config::ToolNames {
            find: original.tools.search_query.clone(),
            ..original.tools.clone()
        },
    ] {
        let mut duplicate = original.clone();
        duplicate.tools = names;
        let file = settings_file(&duplicate);
        assert!(
            config::load(file.path())
                .unwrap_err()
                .client_message()
                .contains("duplicates")
        );
    }
}
#[tokio::test]
async fn unreadable_or_invalid_configuration_does_not_start_with_defaults() {
    let directory = tempfile::tempdir().unwrap();
    let missing = binary()
        .arg("--config")
        .arg(directory.path().join("missing.yaml"))
        .arg("--help")
        .output()
        .await
        .unwrap();
    assert!(!missing.status.success());
    assert_eq!(missing.stdout.as_slice(), b"");
    let mut settings = renamed_config();
    settings.tools.find.clone_from(&settings.tools.open);
    let file = settings_file(&settings);
    let invalid = invoke(&file, &["--help"]).await;
    assert!(!invalid.status.success());
    assert_eq!(invalid.stdout.as_slice(), b"");
    assert!(String::from_utf8_lossy(&invalid.stderr).contains("duplicates"));
}
