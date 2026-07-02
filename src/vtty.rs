//! VTty session manager — lifecycle, I/O, screen state.
//!
//! A [`VttyManager`] spawns real child processes inside pseudo-terminals, pumps
//! the PTY output through the [`crate::screen`] emulator on a background thread,
//! and lets callers type into the program and read back the rendered screen.

use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
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
    pub alive: bool,
    screen: Arc<Mutex<Screen>>,
    pty: Arc<Mutex<Option<pty::Pty>>>,
}

impl VttySession {
    /// Clone the current screen grid (snapshot).
    pub fn screen_snapshot(&self) -> Screen {
        self.screen.lock().unwrap().clone()
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

        // Reader pump: a dedicated OS thread reads PTY output and feeds the
        // screen. It extracts the reader from the shared pty once (so the pty
        // lock isn't held across the blocking read loop).
        {
            let screen = Arc::clone(&screen);
            let pty_arc = Arc::clone(&pty_arc);
            thread::spawn(move || {
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
            alive: true,
            screen,
            pty: pty_arc,
        };
        self.sessions.lock().await.insert(id.clone(), session);
        Ok(id)
    }

    pub async fn kill(&self, id: &str) -> bool {
        let pty_arc = {
            let mut sessions = self.sessions.lock().await;
            let Some(session) = sessions.get_mut(id) else {
                return false;
            };
            session.alive = false;
            Arc::clone(&session.pty)
        };
        if let Ok(mut guard) = pty_arc.lock() {
            if let Some(pty) = guard.as_mut() {
                let _ = pty.child.kill();
                let _ = pty.child.wait();
            }
            *guard = None; // drops master → reader thread sees EOF
        }
        true
    }

    pub async fn list(&self) -> Vec<(VttySessionId, bool)> {
        self.sessions
            .lock()
            .await
            .iter()
            .map(|(id, s)| (id.clone(), s.alive))
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
            session
                .screen
                .lock()
                .unwrap()
                .resize(cols as usize, rows as usize);
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

/// A `Read` that is always at EOF — placeholder once the real reader has been
/// moved out of a `Pty` into the pump thread.
struct EmptyRead;
impl Read for EmptyRead {
    fn read(&mut self, _buf: &mut [u8]) -> std::io::Result<usize> {
        Ok(0)
    }
}
