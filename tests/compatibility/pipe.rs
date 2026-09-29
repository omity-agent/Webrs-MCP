use super::wire::{LEGACY, MODERN, assert_tools, initialize, request};
use core::time::Duration;
use rmcp::serde_json::{Value, json};
use std::process::Stdio;
use tokio::{
    io::{AsyncBufReadExt as _, AsyncWriteExt as _, BufReader},
    process::{Child, ChildStdin, ChildStdout, Command},
    time::timeout,
};
pub(super) struct Process {
    child: Child,
    input: ChildStdin,
    output: BufReader<ChildStdout>,
}
impl Process {
    pub(super) fn start(mode: &str) -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_web-rs"))
            .args(["--transport", "stdio", "--mode", mode])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true)
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
    pub(super) async fn send(&mut self, body: &Value) {
        let mut bytes = sonic_rs::to_vec(body).unwrap();
        bytes.push(b'\n');
        self.input.write_all(&bytes).await.unwrap();
        self.input.flush().await.unwrap();
    }
    pub(super) async fn call(&mut self, body: &Value) -> Value {
        self.send(body).await;
        let mut line = String::new();
        let size = timeout(Duration::from_secs(15), self.output.read_line(&mut line))
            .await
            .unwrap()
            .unwrap();
        assert!(size > 0, "stdio closed without a response");
        sonic_rs::from_str(&line).unwrap()
    }
    pub(super) async fn close(mut self) {
        drop(self.input);
        let status = timeout(Duration::from_secs(10), self.child.wait())
            .await
            .unwrap()
            .unwrap();
        assert!(status.success(), "{status}");
    }
}
#[tokio::test]
async fn real_stdio_process_negotiates_both_eras_and_work_modes() {
    for mode in ["response", "filesystem"] {
        for version in [LEGACY, MODERN] {
            let mut process = Process::start(mode);
            if version == LEGACY {
                let init = process.call(&initialize(version)).await;
                assert_eq!(
                    init.pointer("/result/protocolVersion"),
                    Some(&json!(version))
                );
                process
                    .send(&json ! ({ "jsonrpc" : "2.0" , "method" : "notifications/initialized" }))
                    .await;
            } else {
                let discover = process
                    .call(&request(version, "server/discover", json!({})))
                    .await;
                assert_eq!(
                    discover.pointer("/result/supportedVersions"),
                    Some(&json!([LEGACY, MODERN]))
                );
            }
            let tools = process
                .call(&request(version, "tools/list", json!({})))
                .await;
            assert_tools(&tools, version, mode == "filesystem");
            let failure = process
                .call(&request(
                    version,
                    "tools/call",
                    json ! ({ "name" : "search_query" , "arguments" : { } }),
                ))
                .await;
            assert!(failure.get("result").is_some(), "{failure}");
            process.close().await;
        }
    }
}
#[tokio::test]
async fn modern_stdio_can_start_with_a_tool_request_and_requires_metadata_afterwards() {
    let mut process = Process::start("response");
    let tools = process
        .call(&request(MODERN, "tools/list", json!({})))
        .await;
    assert_tools(&tools, MODERN, false);
    let missing = process
        .call(&request(LEGACY, "tools/list", json!({})))
        .await;
    assert_eq!(missing.pointer("/error/code"), Some(&json!(-32602_i64)));
    let valid = process
        .call(&request(MODERN, "tools/list", json!({})))
        .await;
    assert_tools(&valid, MODERN, false);
    process.close().await;
}
#[tokio::test]
async fn stdio_negotiation_does_not_leave_the_requested_unsupported_version_in_context() {
    let mut process = Process::start("response");
    let init = process.call(&initialize(MODERN)).await;
    assert_eq!(
        init.pointer("/result/protocolVersion"),
        Some(&json!(LEGACY))
    );
    process
        .send(&json ! ({ "jsonrpc" : "2.0" , "method" : "notifications/initialized" }))
        .await;
    let tools = process
        .call(&request(LEGACY, "tools/list", json!({})))
        .await;
    assert_tools(&tools, LEGACY, false);
    process.close().await;
}
