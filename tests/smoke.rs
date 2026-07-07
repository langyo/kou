//! End-to-end PTY smoke tests for the VttyManager API the tairitsu-mcp wrapper
//! delegates to. Each test spawns a *real* child process in a PTY, drives it,
//! and reads the rendered screen back — so a pass means the launch / send /
//! query / kill surface actually works on this host.
//!
//! These tests spawn Unix shell commands (`echo`, `bash`, `printf`, `sleep`),
//! so they are compiled out on non-Unix targets.

#![cfg(unix)]

use std::time::{Duration, Instant};

use kou::VttyManager;

/// Poll the screen until it contains `pattern`, or `timeout` elapses.
async fn wait_for_text(mgr: &VttyManager, id: &str, pattern: &str, timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    loop {
        if let Ok(t) = mgr.screenshot(id).await {
            if t.contains(pattern) {
                return true;
            }
        }
        if Instant::now() > deadline {
            return false;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

/// Poll until the session has any non-blank output.
async fn wait_for_output(mgr: &VttyManager, id: &str, timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    loop {
        if mgr.has_output(id).await.unwrap_or(false) {
            return true;
        }
        if Instant::now() > deadline {
            return false;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

#[tokio::test]
async fn launch_screenshot_find_kill() {
    let mgr = VttyManager::new();
    let info = mgr
        .launch("echo hello-vtty", None, &[], 80, 24, Some("smoke"))
        .await
        .expect("launch");
    assert!(info.alive);
    assert_eq!(info.name, "smoke");
    assert_eq!(info.cols, 80);
    assert_eq!(info.rows, 24);

    let id = info.id.clone();
    assert!(
        wait_for_output(&mgr, &id, Duration::from_secs(5)).await,
        "screen should have output"
    );

    let text = mgr.screenshot(&id).await.unwrap();
    assert!(text.contains("hello-vtty"), "got: {text}");

    let hits = mgr.find_text(&id, "hello").await.unwrap();
    assert!(!hits.is_empty());

    let pinged = mgr.ping(&id).await.unwrap();
    assert_eq!(pinged.id, id);
    assert_eq!(mgr.list().await.len(), 1);

    let killed = mgr.kill(&id).await.expect("kill returns info");
    assert_eq!(killed.id, id);
    assert!(mgr.ping(&id).await.is_none(), "session removed after kill");
}

#[tokio::test]
async fn send_text_and_keys() {
    let mgr = VttyManager::new();
    let info = mgr
        .launch("bash --norc --noprofile", None, &[], 80, 24, None)
        .await
        .expect("launch bash");
    let id = info.id.clone();
    tokio::time::sleep(Duration::from_millis(400)).await;

    mgr.send_text(&id, "echo SEND_KEYS_OK").await.unwrap();
    mgr.send_keys(&id, "ENTER").await.unwrap();

    assert!(
        wait_for_text(&mgr, &id, "SEND_KEYS_OK", Duration::from_secs(5)).await,
        "typed command + Enter should be echoed"
    );
    let _ = mgr.kill(&id).await;
}

#[tokio::test]
async fn launch_with_env() {
    let mgr = VttyManager::new();
    let env: Vec<(String, String)> = vec![("KOU_SMOKE_VAR".into(), "xyz123".into())];
    let env_refs: Vec<(&str, &str)> = env.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
    let info = mgr
        .launch("printenv KOU_SMOKE_VAR", None, &env_refs, 80, 24, None)
        .await
        .expect("launch printenv");
    let id = info.id.clone();

    assert!(
        wait_for_text(&mgr, &id, "xyz123", Duration::from_secs(5)).await,
        "child should inherit the overridden env var"
    );
    let _ = mgr.kill(&id).await;
}

#[tokio::test]
async fn scrollback_captures_scrolled_lines() {
    let mgr = VttyManager::new();
    // 3-row screen: writing 5 lines scrolls the first two into scrollback.
    let info = mgr
        .launch("printf 'l0\\nl1\\nl2\\nl3\\nl4\\n'", None, &[], 40, 3, None)
        .await
        .expect("launch");
    let id = info.id.clone();
    assert!(
        wait_for_text(&mgr, &id, "l4", Duration::from_secs(5)).await,
        "last line should be visible"
    );

    let sb = mgr.scrollback(&id).await.unwrap();
    assert!(
        sb.contains("l0"),
        "scrollback should retain early lines: {sb}"
    );
    assert!(
        sb.contains("l4"),
        "scrollback+screen should include current: {sb}"
    );
    let _ = mgr.kill(&id).await;
}

#[tokio::test]
async fn resize_updates_dimensions() {
    let mgr = VttyManager::new();
    let info = mgr
        .launch("sleep 5", None, &[], 80, 24, None)
        .await
        .expect("launch");
    let id = info.id.clone();
    mgr.resize(&id, 120, 40).await.unwrap();
    let pinged = mgr.ping(&id).await.unwrap();
    assert_eq!(pinged.cols, 120);
    assert_eq!(pinged.rows, 40);
    let _ = mgr.kill(&id).await;
}
