//! Stdio smoke test for the `kou mcp` MCP server.
//!
//! Boots the server as a subprocess, performs the MCP `initialize` handshake,
//! then issues `tools/list` and asserts the full expected vtty tool roster is
//! advertised. It does *not* invoke any tool — the point is to prove the server
//! starts, speaks the protocol, and exposes the right surface. (Live tool
//! invocation is exercised by the PTY smoke tests against VttyManager.)
//!
//! Gated behind the `mcp` feature since the subcommand only exists with it.

#![cfg(feature = "mcp")]

use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

/// Build (or reuse) the `kou` binary and return the path to it.
fn kou_binary() -> std::path::PathBuf {
    // CARGO_BIN_EXE_kou is set by cargo for integration tests when a [[bin]]
    // named `kou` exists, pointing at the built artifact.
    std::path::PathBuf::from(env!("CARGO_BIN_EXE_kou"))
}

/// Send one JSON-RPC message as newline-delimited JSON (JSONL) — the framing
/// rmcp's stdio transport speaks. `id == None` ⇒ a notification (no id field).
fn write_msg<W: Write>(w: &mut W, id: Option<u64>, method: &str, params: serde_json::Value) {
    let mut msg = serde_json::json!({
        "jsonrpc": "2.0",
        "method": method,
        "params": params,
    });
    if let Some(id) = id {
        msg["id"] = serde_json::json!(id);
    }
    serde_json::to_writer(&mut *w, &msg).unwrap();
    writeln!(&mut *w).unwrap();
    w.flush().unwrap();
}

/// Read JSONL lines until a response with the expected id arrives (or the
/// deadline passes). Notifications are skipped.
fn read_response<R: BufRead>(r: &mut R, want_id: u64, timeout: Duration) -> serde_json::Value {
    let deadline = Instant::now() + timeout;
    let mut line = String::new();
    loop {
        if Instant::now() > deadline {
            panic!("timed out waiting for response id={want_id}");
        }
        line.clear();
        let n = r.read_line(&mut line).expect("read line");
        if n == 0 {
            panic!("server closed stdout before responding to id={want_id}");
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let v: serde_json::Value = match serde_json::from_str(trimmed) {
            Ok(v) => v,
            Err(_) => continue, // partial / non-JSON noise on stderr-mixed pipes
        };
        if v.get("id").and_then(|i| i.as_u64()) == Some(want_id) {
            return v;
        }
        // Otherwise it's a notification (e.g. initialized ack); keep reading.
    }
}

#[test]
fn mcp_server_lists_vtty_tools() {
    let bin = kou_binary();
    let mut child = Command::new(&bin)
        .arg("mcp")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn `kou mcp` ({bin:?}): {e}"));

    let mut stdin = child.stdin.take().unwrap();
    let mut stdout = BufReader::new(child.stdout.take().unwrap());

    // initialize — client declares it wants the tool capability.
    write_msg(
        &mut stdin,
        Some(1),
        "initialize",
        serde_json::json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": { "name": "kou-mcp-smoke", "version": "0.0.0" },
        }),
    );
    let init = read_response(&mut stdout, 1, Duration::from_secs(20));
    assert_eq!(
        init["result"]["protocolVersion"].as_str(),
        Some("2024-11-05"),
        "initialize response: {init}"
    );
    let caps = &init["result"]["capabilities"];
    assert!(
        caps.get("tools").is_some(),
        "server did not advertise tools capability: {caps}"
    );

    // notifications/initialized ack (no response expected).
    write_msg(&mut stdin, None, "notifications/initialized", serde_json::json!({}));

    // tools/list — assert the full vtty roster.
    write_msg(&mut stdin, Some(2), "tools/list", serde_json::json!({}));
    let list = read_response(&mut stdout, 2, Duration::from_secs(20));
    let tools: Vec<String> = list["result"]["tools"]
        .as_array()
        .expect("tools is an array")
        .iter()
        .map(|t| t["name"].as_str().unwrap().to_string())
        .collect();

    for expected in [
        "vtty_launch",
        "vtty_kill",
        "vtty_send_keys",
        "vtty_send_text",
        "vtty_screenshot",
        "vtty_wait",
        "vtty_ready",
        "vtty_scrollback",
        "vtty_resize",
        "vtty_list",
        "vtty_ping",
    ] {
        assert!(
            tools.iter().any(|t| t == expected),
            "missing tool `{expected}`; got: {tools:?}"
        );
    }

    drop(stdin);
    // Give the server a moment to exit on its own (stdin closed → shutdown),
    // then force-kill if it's still running. `wait_timeout` is not stable on
    // all platforms, so we poll with try_wait on a short deadline.
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        if child.try_wait().map(|o| o.is_some()).unwrap_or(false) {
            break;
        }
        thread::sleep(Duration::from_millis(50));
    }
    let _ = child.kill();
}
