use crate::envelope::{STATELESS, request};
use core::time::Duration;
use rmcp::{serde_json::Value, service::serve_directly};
use tokio::io::{AsyncBufReadExt as _, AsyncWriteExt as _, BufReader, duplex};
use web_rs::mcp::tools::ToolService;
pub(crate) async fn call(service: ToolService, method: &str, params: Value) -> Value {
    let (client_io, server_io) = duplex(0x0001_0000);
    let running = serve_directly(service, server_io, None);
    let mut client = BufReader::new(client_io);
    let mut encoded = sonic_rs::to_vec(&request(STATELESS, method, params)).unwrap();
    encoded.push(b'\n');
    client.write_all(&encoded).await.unwrap();
    client.flush().await.unwrap();
    let mut line = String::new();
    let size = tokio::time::timeout(Duration::from_secs(10), client.read_line(&mut line))
        .await
        .unwrap()
        .unwrap();
    assert!(size > 0, "in-memory transport closed without a response");
    let reply = sonic_rs::from_str(&line).unwrap();
    running.cancel().await.unwrap();
    reply
}
