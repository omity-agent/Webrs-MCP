use axum::Router;
use core::time::Duration;
use std::{io::Write as _, process::Output};
use tempfile::NamedTempFile;
use tokio::{net::TcpListener, process::Command, task::JoinHandle, time::timeout};
use web_rs::config::{self, AppConfig, ToolNames};
pub(super) const PAGE: &str = "# Sample\nA&B <raw>\nThe needle is here.\n";
pub(super) struct Site {
    pub(super) address: String,
    pub(super) task: JoinHandle<()>,
}
impl Site {
    pub async fn listen(router: Router) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = format!("http://{}", listener.local_addr().unwrap());
        let task = tokio::spawn(async move {
            axum::serve(listener, router).await.unwrap();
        });
        Self { address, task }
    }
}
impl Drop for Site {
    fn drop(&mut self) {
        self.task.abort();
    }
}
pub(super) fn renamed_config() -> AppConfig {
    let mut settings = config::load_embedded().unwrap();
    settings.tools = ToolNames {
        search_query: "lookup".to_owned(),
        open: "browse".to_owned(),
        find: "scan".to_owned(),
    };
    settings
}
pub(super) fn settings_file(settings: &AppConfig) -> NamedTempFile {
    let mut file = tempfile::Builder::new().suffix(".yaml").tempfile().unwrap();
    file.write_all(serde_saphyr::to_string(settings).unwrap().as_bytes())
        .unwrap();
    file
}
pub(super) fn binary() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_web-rs"));
    command.env_remove("RUST_LOG").kill_on_drop(true);
    command
}
pub(super) async fn invoke(file: &NamedTempFile, arguments: &[&str]) -> Output {
    timeout(
        Duration::from_secs(20),
        binary()
            .arg("--config")
            .arg(file.path())
            .args(arguments)
            .output(),
    )
    .await
    .unwrap()
    .unwrap()
}
pub(super) fn stdout(output: &Output) -> &str {
    core::str::from_utf8(&output.stdout).unwrap()
}
pub(super) fn assert_success(output: &Output) {
    assert!(
        output.status.success(),
        "{}: {}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
}
