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
use tokio::sync::Mutex as AsyncMutex;

use crate::pty;
use crate::screen::Screen;

pub type VttySessionId = String;

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
}

impl VttySession {
    pub fn alive(&self) -> bool {
        self.alive.load(Ordering::SeqCst)
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
    pub async fn launch(
        &self,
        command: &str,
        cwd: Option<&str>,
        cols: u16,
        rows: u16,
    ) -> Result<VttySessionId> {
        let cwd_path = cwd.map(PathBuf::from);
        let pty_handle = pty::spawn(command, cwd_path.as_deref(), cols, rows)?;

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
            name: None,
            command: command.to_string(),
            cols,
            rows,
            alive,
            screen,
            pty: pty_arc,
        };
        self.sessions.lock().await.insert(id.clone(), session);
        Ok(id)
    }

    pub async fn kill(&self, id: &str) -> bool {
        let session = {
            let mut sessions = self.sessions.lock().await;
            // Remove (and later Drop) the session so its child is reaped and
            // its pump thread exits, instead of lingering as alive=false.
            match sessions.remove(id) {
                Some(s) => s,
                None => return false,
            }
        };
        // Drop reaps the child + closes the PTY; call shutdown explicitly too
        // for clarity (idempotent).
        session.shutdown();
        true
    }

    pub async fn list(&self) -> Vec<(VttySessionId, bool)> {
        self.sessions
            .lock()
            .await
            .iter()
            .map(|(id, s)| (id.clone(), s.alive()))
            .collect()
    }

    /// Write `text` to the session's PTY (i.e. type into the program).
    pub async fn send_text(&self, id: &str, text: &str) -> Result<()> {
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
            .write_all(text.as_bytes())
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
