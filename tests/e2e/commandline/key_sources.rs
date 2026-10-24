use crate::{
    listener::LocalServer,
    process::{invoke, invoke_with_env, renamed_settings, settings_file},
};
use axum::{Router, routing::get};
#[tokio::test]
async fn missing_conflicting_and_empty_api_key_variables_stop_before_network_io() {
    let server =
        LocalServer::start(Router::new().route("/page", get(|| async { "unexpected" }))).await;
    let file = settings_file(&renamed_settings());
    let url = format!("{}/page", server.address);
    for (arguments, environment, expected) in [
        (vec!["lookup", "test"], vec![], "EXA_API_KEY"),
        (
            vec!["lookup", "test"],
            vec![
                ("EXA_API_KEY", "secret-exa"),
                ("OCTEN_API_KEY", "secret-octen"),
            ],
            "OCTEN_API_KEY",
        ),
        (
            vec!["lookup", "test"],
            vec![("EXA_API_KEY", "")],
            "EXA_API_KEY",
        ),
        (
            vec!["lookup", "test"],
            vec![("EXA_API_KEY", " \t")],
            "EXA_API_KEY",
        ),
        (
            vec!["lookup", "test"],
            vec![("OCTEN_API_KEY", "")],
            "OCTEN_API_KEY",
        ),
        (
            vec!["browse", &url],
            vec![
                ("JINA_API_KEY", "secret-jina"),
                ("TINYFISH_API_KEY", "secret-tinyfish"),
            ],
            "TINYFISH_API_KEY",
        ),
        (
            vec!["browse", &url],
            vec![("JINA_API_KEY", "")],
            "JINA_API_KEY",
        ),
        (
            vec!["browse", &url],
            vec![("TINYFISH_API_KEY", " ")],
            "TINYFISH_API_KEY",
        ),
    ] {
        let output = invoke_with_env(file.path(), &arguments, &environment).await;
        assert!(
            !output.status.success() && output.stdout.is_empty(),
            "{output:?}"
        );
        let diagnostic = String::from_utf8_lossy(&output.stderr);
        assert!(diagnostic.contains(expected), "{diagnostic}");
        assert!(!diagnostic.contains("secret-"), "{diagnostic}");
    }
    assert!(server.visits.lock().await.is_empty());
}
#[tokio::test]
async fn invalid_and_duplicate_api_key_variable_names_stop_configuration_loading() {
    for name in ["", "1KEY", "HAS SPACE", "KEY=VALUE", "exa_api_key"] {
        let mut settings = renamed_settings();
        settings.environment.octen = name.to_owned();
        let file = settings_file(&settings);
        let output = invoke(file.path(), &["--help"]).await;
        assert!(
            !output.status.success() && output.stdout.is_empty(),
            "{name}"
        );
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("environment.octen"),
            "{output:?}"
        );
    }
}
