use serde_json::json;
use std::process::Stdio;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::Command,
};

#[tokio::test]
async fn debug_logging_never_records_mcp_payloads_or_device_identifiers() {
    let mut process = Command::new(env!("CARGO_BIN_EXE_andromeda"))
        .env("RUST_LOG", "trace")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut input = process.stdin.take().unwrap();
    let mut output = BufReader::new(process.stdout.take().unwrap());
    let requests = [
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"privacy-test","version":"1"}}}),
        json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
        json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"adm_type_text","arguments":{"serial":"PRIVATE_DEVICE_SENTINEL","text":"PRIVATE_TEXT_SENTINEL"}}}),
    ];
    for (i, request) in requests.iter().enumerate() {
        input
            .write_all(format!("{request}\n").as_bytes())
            .await
            .unwrap();
        if i != 1 {
            let mut line = String::new();
            tokio::time::timeout(
                std::time::Duration::from_secs(3),
                output.read_line(&mut line),
            )
            .await
            .unwrap()
            .unwrap();
            let result: serde_json::Value = serde_json::from_str(&line).unwrap();
            if i == 2 {
                assert_eq!(
                    result["result"]["structuredContent"]["error"]["code"],
                    "control_disabled"
                );
            }
        }
    }
    drop(input);
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(3),
        process.wait_with_output(),
    )
    .await
    .unwrap()
    .unwrap();
    let logs = String::from_utf8_lossy(&result.stderr);
    assert!(
        !logs.contains("PRIVATE_TEXT_SENTINEL"),
        "MCP text leaked into logs"
    );
    assert!(
        !logs.contains("PRIVATE_DEVICE_SENTINEL"),
        "Device identifier leaked into logs"
    );
}
