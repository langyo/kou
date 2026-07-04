//! VTty session manager — lifecycle, I/O, screen state.
//!
//! A [`VttyManager`] spawns real child processes inside pseudo-terminals, pumps
//! the PTY output through the [`crate::screen`] emulator on a background thread,
//! and lets callers type into the program and read back the rendered screen.

use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

use anyhow::Result;
use serde::Serialize;
use tokio::sync::Mutex as AsyncMutex;

use crate::pty;
use crate::screen::Screen;

pub type VttySessionId = String;

/// Serializable snapshot of one session's identity + liveness, the shape the
/// MCP `launch` / `kill` / `list` / `ping` tools return.
#[derive(Debug, Clone, Serialize)]
pub struct SessionInfo {
    pub id: VttySessionId,
    pub name: String,
    pub command: String,
    pub cols: u16,
    pub rows: u16,
    pub alive: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pid: Option<u32>,
}

static SESSION_SEQ: AtomicU64 = AtomicU64::new(0);

pub struct VttySession {
    pub id: VttySessionId,
    pub name: Option<String>,
    pub command: String,
    pub cols: u16,
    pub rows: u16,
    alive: Arc<AtomicBool>,
    screen: Arc<Mutex<Screen>>,
    pty: Arc<Mutex<Option<pty::Pty>>>,
    pid: Option<u32>,
}

impl VttySession {
    pub fn alive(&self) -> bool {
        self.alive.load(Ordering::SeqCst)
    }

    fn info(&self) -> SessionInfo {
        SessionInfo {
            id: self.id.clone(),
            name: self.name.clone().unwrap_or_default(),
            command: self.command.clone(),
            cols: self.cols,
            rows: self.rows,
            alive: self.alive(),
            pid: self.pid,
        }
    }

    /// Clone the current screen grid (snapshot).
    pub fn screen_snapshot(&self) -> Screen {
        // Tolerate a poisoned mutex (a panic during feed) rather than
        // propagating it into the caller's async task.
        self.screen
            .lock()
            .map(|s| s.clone())
            .unwrap_or_else(|p| p.into_inner().clone())
    }

    /// Reap the child and drop the PTY master so the reader pump sees EOF and
    /// exits. Called from `kill` and from `Drop`.
    fn shutdown(&self) {
        if let Ok(mut guard) = self.pty.lock() {
            if let Some(pty) = guard.as_mut() {
                let _ = pty.child.kill();
                let _ = pty.child.wait();
            }
            // Dropping the Pty drops the master → the pump thread's read()
            // returns 0 (EOF) and the thread exits. No more leaked threads/fds.
            *guard = None;
        }
    }
}

impl Drop for VttySession {
    fn drop(&mut self) {
        self.shutdown();
    }
}

pub struct VttyManager {
    sessions: Arc<AsyncMutex<HashMap<VttySessionId, VttySession>>>,
}

impl VttyManager {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(AsyncMutex::new(HashMap::new())),
        }
    }

    /// Spawn `command` (program + args, whitespace-split) in a PTY and start a
    /// background reader thread that feeds its output into the screen emulator.
    ///
    /// `env` adds/overrides environment variables on top of the parent's env;
    /// pass `&[]` to inherit unchanged. `name` is an optional human label
    /// surfaced in [`SessionInfo`].
    pub async fn launch(
        &self,
        command: &str,
        cwd: Option<&str>,
        env: &[(&str, &str)],
        cols: u16,
        rows: u16,
        name: Option<&str>,
    ) -> Result<SessionInfo> {
        let cwd_path = cwd.map(PathBuf::from);
        let pty_handle = pty::spawn(command, cwd_path.as_deref(), env, cols, rows)?;
        let pid = pty_handle.pid;

        let screen = Arc::new(Mutex::new(Screen::new(cols as usize, rows as usize)));
        let pty_arc = Arc::new(Mutex::new(Some(pty_handle)));
        let alive = Arc::new(AtomicBool::new(true));

        // Reader pump: a dedicated OS thread reads PTY output and feeds the
        // screen. It extracts the reader from the shared pty once (so the pty
        // lock isn't held across the blocking read loop). When the loop exits
        // (EOF / error) it sets `alive` to false so callers get a truthful
        // view of the session state.
        {
            let screen = Arc::clone(&screen);
            let pty_arc = Arc::clone(&pty_arc);
            let alive = Arc::clone(&alive);
            thread::spawn(move || {
                let _guard = AliveGuard(alive);
                let mut reader = {
                    let mut guard = match pty_arc.lock() {
                        Ok(g) => g,
                        Err(_) => return,
                    };
                    let Some(pty) = guard.as_mut() else {
                        return;
                    };
                    std::mem::replace(&mut pty.reader, Box::new(EmptyRead))
                };
                let mut buf = [0u8; 8192];
                loop {
                    match reader.read(&mut buf) {
                        Ok(0) => break, // EOF — child exited / pty closed
                        Ok(n) => {
                            if let Ok(mut s) = screen.lock() {
                                s.feed(&buf[..n]);
                            }
                        }
                        Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                        Err(_) => break,
                    }
                }
            });
        }

        let id = format!(
            "kou-{}-{}",
            std::process::id(),
            SESSION_SEQ.fetch_add(1, Ordering::Relaxed)
        );

        let session = VttySession {
            id: id.clone(),
            name: name.map(|s| s.to_string()),
            command: command.to_string(),
            cols,
            rows,
            alive,
            screen,
            pty: pty_arc,
            pid,
        };
        let info = session.info();
        self.sessions.lock().await.insert(id.clone(), session);
        Ok(info)
    }

    pub async fn kill(&self, id: &str) -> Option<SessionInfo> {
        let session = {
            let mut sessions = self.sessions.lock().await;
            // Remove (and later Drop) the session so its child is reaped and
            // its pump thread exits, instead of lingering as alive=false.
            match sessions.remove(id) {
                Some(s) => s,
                None => return None,
            }
        };
        let info = session.info();
        // Drop reaps the child + closes the PTY; call shutdown explicitly too
        // for clarity (idempotent).
        session.shutdown();
        Some(info)
    }

    pub async fn list(&self) -> Vec<SessionInfo> {
        self.sessions
            .lock()
            .await
            .values()
            .map(|s| s.info())
            .collect()
    }

    /// Refresh + return a session's [`SessionInfo`]. `None` if not found.
    pub async fn ping(&self, id: &str) -> Option<SessionInfo> {
        self.sessions.lock().await.get(id).map(|s| s.info())
    }

    /// Write `text` to the session's PTY (i.e. type into the program). Line
    /// feeds are translated to carriage returns the way a real terminal sends
    /// Enter (CRLF collapses to a single CR).
    pub async fn send_text(&self, id: &str, text: &str) -> Result<()> {
        let encoded = encode_input(text);
        self.write_bytes(id, &encoded).await
    }

    /// Type a space-separated sequence of named keys (e.g. `"ENTER"`,
    /// `"CTRL+C"`, `"UP"`) into the session. See [`parse_keys`] for the
    /// accepted names.
    pub async fn send_keys(&self, id: &str, keys: &str) -> Result<()> {
        let bytes = parse_keys(keys)?;
        self.write_bytes(id, &bytes).await
    }

    async fn write_bytes(&self, id: &str, bytes: &[u8]) -> Result<()> {
        let pty_arc = {
            let sessions = self.sessions.lock().await;
            let session = sessions
                .get(id)
                .ok_or_else(|| anyhow::anyhow!("session {} not found", id))?;
            Arc::clone(&session.pty)
        };
        let mut guard = pty_arc
            .lock()
            .map_err(|e| anyhow::anyhow!("pty lock poisoned: {e}"))?;
        let Some(pty) = guard.as_mut() else {
            anyhow::bail!("session {id} has no live PTY");
        };
        pty.writer
            .write_all(bytes)
            .map_err(|e| anyhow::anyhow!("pty write failed: {e}"))?;
        Ok(())
    }

    /// Plain-text snapshot of the screen.
    pub async fn screenshot(&self, id: &str) -> Result<String> {
        let sessions = self.sessions.lock().await;
        let session = sessions
            .get(id)
            .ok_or_else(|| anyhow::anyhow!("session {} not found", id))?;
        Ok(session.screen_snapshot().text())
    }

    /// Return a snapshot of the session's screen grid (cloned), for renderers
    /// that need the cell attributes, not just the plain text.
    pub async fn screen(&self, id: &str) -> Result<Screen> {
        let sessions = self.sessions.lock().await;
        let session = sessions
            .get(id)
            .ok_or_else(|| anyhow::anyhow!("session {} not found", id))?;
        Ok(session.screen_snapshot())
    }

    /// `true` if the session's screen has any non-blank cell.
    pub async fn has_output(&self, id: &str) -> Result<bool> {
        let sessions = self.sessions.lock().await;
        let session = sessions
            .get(id)
            .ok_or_else(|| anyhow::anyhow!("session {} not found", id))?;
        Ok(session.screen_snapshot().has_output())
    }

    /// Locations of `pattern` on the current screen (see [`Screen::find_text`]).
    pub async fn find_text(&self, id: &str, pattern: &str) -> Result<Vec<(usize, usize)>> {
        let sessions = self.sessions.lock().await;
        let session = sessions
            .get(id)
            .ok_or_else(|| anyhow::anyhow!("session {} not found", id))?;
        Ok(session.screen_snapshot().find_text(pattern))
    }

    /// Scrollback buffer concatenated with the current screen.
    pub async fn scrollback(&self, id: &str) -> Result<String> {
        let sessions = self.sessions.lock().await;
        let session = sessions
            .get(id)
            .ok_or_else(|| anyhow::anyhow!("session {} not found", id))?;
        Ok(session.screen_snapshot().scrollback_with_screen())
    }

    pub async fn resize(&self, id: &str, cols: u16, rows: u16) -> Result<()> {
        let pty_arc = {
            let mut sessions = self.sessions.lock().await;
            let session = sessions
                .get_mut(id)
                .ok_or_else(|| anyhow::anyhow!("session {} not found", id))?;
            // Poison-tolerant: resize even if a prior feed panicked.
            if let Ok(mut s) = session.screen.lock() {
                s.resize(cols as usize, rows as usize);
            }
            session.cols = cols;
            session.rows = rows;
            Arc::clone(&session.pty)
        };
        if let Ok(mut guard) = pty_arc.lock() {
            if let Some(pty) = guard.as_mut() {
                let _ = pty.resize(cols, rows);
            }
        }
        Ok(())
    }
}

impl Default for VttyManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Sets `alive` to `false` when dropped — ensures the session's alive flag
/// reflects reality even if the reader thread panics or returns early.
struct AliveGuard(Arc<AtomicBool>);
impl Drop for AliveGuard {
    fn drop(&mut self) {
        self.0.store(false, Ordering::SeqCst);
    }
}

/// A `Read` that is always at EOF — placeholder once the real reader has been
/// moved out of a `Pty` into the pump thread.
struct EmptyRead;
impl Read for EmptyRead {
    fn read(&mut self, _buf: &mut [u8]) -> std::io::Result<usize> {
        Ok(0)
    }
}

// ────────────────────────────────────────────────────────────
// Input encoding — map caller-friendly key names to terminal bytes
// ────────────────────────────────────────────────────────────

/// Translate a text string into PTY input bytes: CRLF and LF both become a
/// single CR (how a terminal signals Enter).
pub fn encode_input(text: &str) -> Vec<u8> {
    text.replace("\r\n", "\r").replace('\n', "\r").into_bytes()
}

/// Parse a space-separated key sequence into the raw bytes a terminal expects.
///
/// Recognised tokens: `ENTER`/`RETURN`, `TAB`, `ESC(APE)`, `BACKSPACE`/`BS`,
/// `DELETE`/`DEL`, arrow keys (`UP`/`DOWN`/`LEFT`/`RIGHT`), `HOME`/`END`,
/// `PAGEUP`/`PAGEDOWN`, `INSERT`, `F1`..`F12`, `SPACE`, `CTRL+<letter>`,
/// `ALT+<chars>`, `SHIFT+<chars>`. Any other token is sent through literally,
/// byte-for-byte.
pub fn parse_keys(keys_str: &str) -> Result<Vec<u8>> {
    let mut buf = Vec::new();
    for part in keys_str.split(' ') {
        let upper = part.to_uppercase();
        match upper.as_str() {
            "ENTER" | "RETURN" => buf.extend(b"\r"),
            "TAB" => buf.extend(b"\t"),
            "ESCAPE" | "ESC" => buf.extend(b"\x1b"),
            "BACKSPACE" | "BS" => buf.extend(b"\x7f"),
            "DELETE" | "DEL" => buf.extend(b"\x1b[3~"),
            "UP" => buf.extend(b"\x1b[A"),
            "DOWN" => buf.extend(b"\x1b[B"),
            "RIGHT" => buf.extend(b"\x1b[C"),
            "LEFT" => buf.extend(b"\x1b[D"),
            "HOME" => buf.extend(b"\x1b[H"),
            "END" => buf.extend(b"\x1b[F"),
            "PAGEUP" | "PAGE_UP" => buf.extend(b"\x1b[5~"),
            "PAGEDOWN" | "PAGE_DOWN" => buf.extend(b"\x1b[6~"),
            "INSERT" => buf.extend(b"\x1b[2~"),
            "F1" => buf.extend(b"\x1bOP"),
            "F2" => buf.extend(b"\x1bOQ"),
            "F3" => buf.extend(b"\x1bOR"),
            "F4" => buf.extend(b"\x1bOS"),
            "F5" => buf.extend(b"\x1b[15~"),
            "F6" => buf.extend(b"\x1b[17~"),
            "F7" => buf.extend(b"\x1b[18~"),
            "F8" => buf.extend(b"\x1b[19~"),
            "F9" => buf.extend(b"\x1b[20~"),
            "F10" => buf.extend(b"\x1b[21~"),
            "F11" => buf.extend(b"\x1b[23~"),
            "F12" => buf.extend(b"\x1b[24~"),
            "SPACE" => buf.push(b' '),
            s if s.starts_with("CTRL+") => {
                let ch = part[5..]
                    .chars()
                    .next()
                    .ok_or_else(|| anyhow::anyhow!("no letter after CTRL+ in '{part}'"))?;
                if ch.is_ascii_alphabetic() {
                    buf.push(ch.to_ascii_uppercase() as u8 - b'A' + 1);
                } else {
                    anyhow::bail!("invalid CTRL+key: {part}");
                }
            }
            s if s.starts_with("ALT+") => {
                buf.push(0x1b);
                for ch in part[4..].chars() {
                    buf.push(ch as u8);
                }
            }
            s if s.starts_with("SHIFT+") => {
                for ch in part[6..].chars() {
                    buf.push(ch as u8);
                }
            }
            _ => {
                for ch in part.chars() {
                    buf.push(ch as u8);
                }
            }
        }
    }
    Ok(buf)
}

#[cfg(test)]
mod key_tests {
    use super::*;

    #[test]
    fn named_keys_map_to_escapes() {
        assert_eq!(parse_keys("ENTER").unwrap(), b"\r");
        assert_eq!(parse_keys("TAB").unwrap(), b"\t");
        assert_eq!(parse_keys("ESC").unwrap(), b"\x1b");
        assert_eq!(parse_keys("UP").unwrap(), b"\x1b[A");
        assert_eq!(parse_keys("F5").unwrap(), b"\x1b[15~");
    }

    #[test]
    fn ctrl_and_alt_prefixes() {
        assert_eq!(parse_keys("CTRL+C").unwrap(), &[0x03]);
        assert_eq!(parse_keys("ALT+F").unwrap(), &[0x1b, b'F']);
    }

    #[test]
    fn literal_text_passes_through() {
        assert_eq!(parse_keys("hello").unwrap(), b"hello");
        assert_eq!(&parse_keys("echo ENTER").unwrap()[..4], b"echo");
    }

    #[test]
    fn encode_input_collapses_newlines() {
        assert_eq!(encode_input("a\nb"), b"a\rb");
        assert_eq!(encode_input("a\r\nb"), b"a\rb");
    }
}
