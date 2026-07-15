// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Integration tests for the JSON-RPC IPC protocol over Unix sockets.
//! These tests spin up a minimal echo-style IPC server and verify the
//! request/response framing without requiring a running daemon.

#[cfg(unix)]
mod unix_ipc {
    use std::time::Duration;
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    use tokio::net::{UnixListener, UnixStream};

    fn temp_socket() -> std::path::PathBuf {
        std::env::temp_dir().join(format!("anvil-test-{}.sock", std::process::id()))
    }

    /// Minimal JSON-RPC echo server that handles health and unknown methods.
    async fn run_echo_server(socket: std::path::PathBuf) {
        if socket.exists() {
            let _ = std::fs::remove_file(&socket);
        }
        let listener = UnixListener::bind(&socket).expect("bind");
        while let Ok((stream, _)) = listener.accept().await {
            tokio::spawn(async move {
                let (reader, mut writer) = stream.into_split();
                let mut lines = BufReader::new(reader).lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    let req: serde_json::Value = match serde_json::from_str(&line) {
                        Ok(v) => v,
                        Err(_) => continue,
                    };
                    let id = req["id"].as_i64().unwrap_or(0);
                    let method = req["method"].as_str().unwrap_or("");
                    let resp = match method {
                        "health" => serde_json::json!({
                            "jsonrpc": "2.0",
                            "id": id,
                            "result": { "status": "ok", "version": "0.1.0" }
                        }),
                        _ => serde_json::json!({
                            "jsonrpc": "2.0",
                            "id": id,
                            "error": { "code": -32601, "message": "method not found" }
                        }),
                    };
                    let _ = writer.write_all(
                        format!("{}\n", serde_json::to_string(&resp).unwrap()).as_bytes(),
                    ).await;
                }
            });
        }
    }

    #[tokio::test]
    async fn health_round_trip() {
        let socket = temp_socket();
        let socket_clone = socket.clone();
        tokio::spawn(run_echo_server(socket_clone));

        tokio::time::sleep(Duration::from_millis(50)).await;

        let stream = UnixStream::connect(&socket).await.expect("connect");
        let (reader, mut writer) = stream.into_split();
        let mut lines = BufReader::new(reader).lines();

        let req = serde_json::json!({ "jsonrpc": "2.0", "id": 1, "method": "health", "params": {} });
        writer.write_all(format!("{}\n", serde_json::to_string(&req).unwrap()).as_bytes()).await.unwrap();

        let response_line = tokio::time::timeout(Duration::from_secs(2), lines.next_line())
            .await
            .expect("timeout")
            .expect("io")
            .expect("line");

        let resp: serde_json::Value = serde_json::from_str(&response_line).expect("json");
        assert_eq!(resp["id"], 1);
        assert_eq!(resp["result"]["status"], "ok");
        assert!(resp["error"].is_null());

        let _ = std::fs::remove_file(&socket);
    }

    #[tokio::test]
    async fn unknown_method_returns_error() {
        let socket = temp_socket().with_extension("unknown");
        let socket_clone = socket.clone();
        tokio::spawn(run_echo_server(socket_clone));

        tokio::time::sleep(Duration::from_millis(50)).await;

        let stream = UnixStream::connect(&socket).await.expect("connect");
        let (reader, mut writer) = stream.into_split();
        let mut lines = BufReader::new(reader).lines();

        let req = serde_json::json!({ "jsonrpc": "2.0", "id": 42, "method": "does_not_exist", "params": {} });
        writer.write_all(format!("{}\n", serde_json::to_string(&req).unwrap()).as_bytes()).await.unwrap();

        let response_line = tokio::time::timeout(Duration::from_secs(2), lines.next_line())
            .await
            .expect("timeout")
            .expect("io")
            .expect("line");

        let resp: serde_json::Value = serde_json::from_str(&response_line).expect("json");
        assert_eq!(resp["id"], 42);
        assert_eq!(resp["error"]["code"], -32601);

        let _ = std::fs::remove_file(&socket);
    }

    #[tokio::test]
    async fn multiple_requests_same_connection() {
        let socket = temp_socket().with_extension("multi");
        let socket_clone = socket.clone();
        tokio::spawn(run_echo_server(socket_clone));

        tokio::time::sleep(Duration::from_millis(50)).await;

        let stream = UnixStream::connect(&socket).await.expect("connect");
        let (reader, mut writer) = stream.into_split();
        let mut lines = BufReader::new(reader).lines();

        for i in 1u64..=5 {
            let req = serde_json::json!({ "jsonrpc": "2.0", "id": i, "method": "health", "params": {} });
            writer.write_all(format!("{}\n", serde_json::to_string(&req).unwrap()).as_bytes()).await.unwrap();
            let line = tokio::time::timeout(Duration::from_secs(2), lines.next_line())
                .await.expect("timeout").expect("io").expect("line");
            let resp: serde_json::Value = serde_json::from_str(&line).unwrap();
            assert_eq!(resp["id"], i);
            assert_eq!(resp["result"]["status"], "ok");
        }

        let _ = std::fs::remove_file(&socket);
    }
}
