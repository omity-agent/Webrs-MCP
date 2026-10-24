use core::time::Duration;
use rmcp::serde_json::Value;
use std::{
    io::Write as _,
    process::{Output, Stdio},
};
use tempfile::NamedTempFile;
use tokio::{
    io::{AsyncBufReadExt as _, AsyncWriteExt as _, BufReader},
    process::{Child, ChildStdin, ChildStdout, Command},
    time::timeout,
};
use web_rs::config::{self, AppConfig, ToolNames};
pub(crate) const PAGE: &str = "# Sample\nA&B <raw>\nThe needle is here.\n";
pub(crate) fn renamed_settings() -> AppConfig {
    let mut settings = config::load_embedded().unwrap();
    settings.tools = ToolNames {
        search_query: "lookup".to_owned(),
        open: "browse".to_owned(),
        find: "scan".to_owned(),
    };
    settings
}
pub(crate) fn settings_file(settings: &AppConfig) -> NamedTempFile {
    let mut file = tempfile::Builder::new().suffix(".yaml").tempfile().unwrap();
    file.write_all(serde_saphyr::to_string(settings).unwrap().as_bytes())
        .unwrap();
    file
}
fn binary() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_web"));
    command.env_remove("RUST_LOG").kill_on_drop(true);
    let environment = config::load_embedded().unwrap().environment;
    for name in [
        environment.exa,
        environment.octen,
        environment.jina,
        environment.tinyfish,
    ] {
        command.env_remove(name);
    }
    command
}
pub(crate) async fn invoke(path: &std::path::Path, arguments: &[&str]) -> Output {
    invoke_with_env(path, arguments, &[]).await
}
pub(crate) async fn invoke_with_env(
    path: &std::path::Path,
    arguments: &[&str],
    environment: &[(&str, &str)],
) -> Output {
    timeout(
        Duration::from_secs(20),
        binary()
            .arg("--config")
            .arg(path)
            .args(arguments)
            .envs(environment.iter().copied())
            .output(),
    )
    .await
    .unwrap()
    .unwrap()
}
pub(crate) fn stdout(output: &Output) -> &str {
    core::str::from_utf8(&output.stdout).unwrap()
}
pub(crate) fn assert_success(output: &Output) {
    assert!(
        output.status.success(),
        "{}: {}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
}
pub(crate) fn start_http(file: &NamedTempFile, environment: &[(&str, &str)]) -> Child {
    binary()
        .arg("--config")
        .arg(file.path())
        .arg("mcp")
        .envs(environment.iter().copied())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::inherit())
        .spawn()
        .unwrap()
}
pub(crate) struct Process {
    child: Child,
    input: ChildStdin,
    output: BufReader<ChildStdout>,
}
impl Process {
    pub fn start(file: &NamedTempFile, mode: &str) -> Self {
        Self::start_with_env(file, mode, &[])
    }
    pub fn start_with_env(file: &NamedTempFile, mode: &str, environment: &[(&str, &str)]) -> Self {
        let mut child = binary()
            .arg("--config")
            .arg(file.path())
            .args(["mcp", "--transport", "stdio", "--mode", mode])
            .envs(environment.iter().copied())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap();
        let input = child.stdin.take().unwrap();
        let output = BufReader::new(child.stdout.take().unwrap());
        Self {
            child,
            input,
            output,
        }
    }
    pub async fn call(&mut self, body: &Value) -> Value {
        self.send(body).await;
        let mut line = String::new();
        let size = timeout(Duration::from_secs(15), self.output.read_line(&mut line))
            .await
            .unwrap()
            .unwrap();
        assert!(size > 0, "stdio closed without a response");
        sonic_rs::from_str(&line).unwrap_or_else(|error| panic!("{error}: {line}"))
    }
    pub async fn send(&mut self, body: &Value) {
        let mut bytes = sonic_rs::to_vec(body).unwrap();
        bytes.push(b'\n');
        self.input.write_all(&bytes).await.unwrap();
        self.input.flush().await.unwrap();
    }
    pub async fn close(mut self) {
        drop(self.input);
        let status = timeout(Duration::from_secs(10), self.child.wait())
            .await
            .unwrap()
            .unwrap();
        assert!(status.success(), "{status}");
    }
}
