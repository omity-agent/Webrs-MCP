use super::{
    harness::{PAGE, binary, renamed_config, settings_file},
    publishing::request,
};
use core::time::Duration;
use rmcp::serde_json::{Value, json};
use std::{path::Path, process::Stdio};
use tokio::{
    io::{AsyncBufReadExt as _, AsyncWriteExt as _, BufReader},
    process::{ChildStdin, ChildStdout},
    time::timeout,
};
async fn stdio_reply(
    input: &mut ChildStdin,
    output: &mut BufReader<ChildStdout>,
    method: &str,
    params: Value,
) -> Value {
    let mut encoded = sonic_rs::to_vec(&request(method, params)).unwrap();
    encoded.push(b'\n');
    input.write_all(&encoded).await.unwrap();
    input.flush().await.unwrap();
    let mut line = String::new();
    let size = timeout(Duration::from_secs(20), output.read_line(&mut line))
        .await
        .unwrap()
        .unwrap();
    assert!(size > 0);
    sonic_rs::from_str(&line).unwrap()
}
#[tokio::test]
async fn stdio_filesystem_uses_the_configured_open_name_for_cached_delivery() {
    let directory = tempfile::tempdir().unwrap();
    let cache = directory.path().join("cached");
    tokio::fs::create_dir(&cache).await.unwrap();
    tokio::fs::write(cache.join(".url.txt"), "https://example.org/cached")
        .await
        .unwrap();
    tokio::fs::write(cache.join("content.txt"), PAGE)
        .await
        .unwrap();
    let file = settings_file(&renamed_config());
    let mut child = binary()
        .arg("--config")
        .arg(file.path())
        .args(["--transport", "stdio", "--mode", "filesystem"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut input = child.stdin.take().unwrap();
    let mut output = BufReader::new(child.stdout.take().unwrap());
    let listed = stdio_reply(&mut input, &mut output, "tools/list", json!({})).await;
    let names = listed
        .pointer("/result/tools")
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .map(|tool| tool.get("name").unwrap().as_str().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(names, ["lookup", "browse"]);
    let delivered = stdio_reply (& mut input , & mut output , "tools/call" , json ! ({ "name" : "browse" , "arguments" : { "output_path" : directory . path () . to_str () . unwrap () , "urls" : ["https://example.org/cached"] } })) . await ;
    assert_eq!(
        delivered.pointer("/result/structuredContent/error"),
        Some(&json!(""))
    );
    let path = Path::new(
        delivered
            .pointer("/result/structuredContent/output_path")
            .unwrap()
            .as_str()
            .unwrap(),
    );
    assert_eq!(
        tokio::fs::read_to_string(path.join("0/content.txt"))
            .await
            .unwrap(),
        PAGE
    );
    drop(input);
    let status = timeout(Duration::from_secs(10), child.wait())
        .await
        .unwrap()
        .unwrap();
    assert!(status.success());
}
