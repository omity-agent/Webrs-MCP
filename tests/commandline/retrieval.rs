use super::harness::{PAGE, Site, assert_success, invoke, renamed_config, settings_file, stdout};
use axum::{Router, routing::get};
use rmcp::serde_json::{Value, json};
async fn markdown_site() -> Site {
    Site::listen(Router::new().route(
        "/page",
        get(|| async { ([("content-type", "text/markdown")], PAGE) }),
    ))
    .await
}
#[tokio::test]
async fn open_and_find_run_with_single_urls_in_both_formats() {
    let site = markdown_site().await;
    let file = settings_file(&renamed_config());
    let url = format!("{}/page", site.address);
    for format in ["xml", "json"] {
        let opened = invoke(&file, &["browse", "--url", &url, "--format", format]).await;
        assert_success(&opened);
        let found = invoke(
            &file,
            &[
                "scan",
                "--url",
                &url,
                "--pattern",
                "nee[d]le",
                "--snippet-tokens",
                "20",
                "--format",
                format,
            ],
        )
        .await;
        assert_success(&found);
        if format == "json" {
            let open_response: Value = sonic_rs::from_str(stdout(&opened)).unwrap();
            let find_response: Value = sonic_rs::from_str(stdout(&found)).unwrap();
            assert_eq!(
                open_response.pointer("/results/0/page/content"),
                Some(&json!(PAGE))
            );
            assert_eq!(
                open_response.pointer("/results/0/page/chunk"),
                Some(&json!(0_u64))
            );
            assert_eq!(
                open_response
                    .get("results")
                    .unwrap()
                    .as_array()
                    .unwrap()
                    .len(),
                1
            );
            assert_eq!(
                find_response
                    .get("pages")
                    .unwrap()
                    .as_array()
                    .unwrap()
                    .len(),
                1
            );
            assert!(
                find_response
                    .pointer("/pages/0/matches/0/snippet")
                    .unwrap()
                    .as_str()
                    .unwrap()
                    .contains("needle")
            );
        } else {
            assert!(stdout(&opened).contains("<CONTENT>\n# Sample\nA&B <raw>"));
            assert!(stdout(&found).starts_with("<PAGES>\n"));
            assert!(stdout(&found).contains("<SNIPPET>"));
            assert!(stdout(&found).contains("needle"));
        }
    }
}
#[tokio::test]
async fn failed_open_has_a_formatted_response_and_nonzero_exit_code() {
    let site = markdown_site().await;
    let file = settings_file(&renamed_config());
    let url = format!("{}/missing", site.address);
    for format in ["xml", "json"] {
        let output = invoke(&file, &["browse", "--url", &url, "--format", format]).await;
        assert!(!output.status.success());
        assert_ne!(output.stderr.as_slice(), b"");
        if format == "json" {
            let response: Value = sonic_rs::from_str(stdout(&output)).unwrap();
            assert!(response.pointer("/results/0/error").is_some());
        } else {
            assert!(stdout(&output).contains("<ERROR>"));
        }
    }
}
#[tokio::test]
async fn invalid_regex_is_reported_without_success_output() {
    let file = settings_file(&renamed_config());
    let output = invoke(
        &file,
        &[
            "scan",
            "--url",
            "https://example.org",
            "--pattern",
            "[",
            "--format",
            "json",
        ],
    )
    .await;
    assert!(!output.status.success());
    assert_eq!(output.stdout.as_slice(), b"");
    assert!(String::from_utf8_lossy(&output.stderr).contains("pattern"));
}
