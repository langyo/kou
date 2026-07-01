//! VTty session manager — lifecycle, I/O, screen state.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::pty;
use crate::screen::Screen;

pub type VttySessionId = String;

pub struct VttySession {
    pub id: VttySessionId,
    pub name: Option<String>,
    pub command: String,
    pub cols: u16,
    pub rows: u16,
    pub screen: Screen,
    pub alive: bool,
}

pub struct VttyManager {
    sessions: Arc<Mutex<HashMap<VttySessionId, VttySession>>>,
}

impl VttyManager {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn launch(
        &self,
        command: &str,
        cwd: Option<&str>,
        cols: u16,
        rows: u16,
    ) -> anyhow::Result<VttySessionId> {
        let id = format!("kou-{}", std::process::id());

        // Spawn PTY
        let _handle = pty::open_pty(cols, rows)?;

        // Spawn child process in PTY
        let mut cmd = tokio::process::Command::new("bash");
        if let Some(dir) = cwd {
            cmd.current_dir(dir);
        }
        // TODO: connect child stdio to PTY slave

        let session = VttySession {
            id: id.clone(),
            name: None,
            command: command.to_string(),
            cols,
            rows,
            screen: Screen::new(cols as usize, rows as usize),
            alive: true,
        };

        self.sessions.lock().await.insert(id.clone(), session);
        Ok(id)
    }

    pub async fn kill(&self, id: &str) -> bool {
        let mut sessions = self.sessions.lock().await;
        if let Some(session) = sessions.get_mut(id) {
            session.alive = false;
            true
        } else {
            false
        }
    }

    pub async fn list(&self) -> Vec<(VttySessionId, bool)> {
        self.sessions
            .lock()
            .await
            .iter()
            .map(|(id, s)| (id.clone(), s.alive))
            .collect()
    }

    pub async fn send_text(&self, id: &str, text: &str) -> anyhow::Result<()> {
        let mut sessions = self.sessions.lock().await;
        let session = sessions
            .get_mut(id)
            .ok_or_else(|| anyhow::anyhow!("session {} not found", id))?;

        // TODO: write to PTY
        session.screen.feed(text.as_bytes());
        Ok(())
    }

    pub async fn screenshot(&self, id: &str) -> anyhow::Result<String> {
        let sessions = self.sessions.lock().await;
        let session = sessions
            .get(id)
            .ok_or_else(|| anyhow::anyhow!("session {} not found", id))?;
        Ok(session.screen.text())
    }

    pub async fn resize(&self, id: &str, cols: u16, rows: u16) -> anyhow::Result<()> {
        let mut sessions = self.sessions.lock().await;
        let session = sessions
            .get_mut(id)
            .ok_or_else(|| anyhow::anyhow!("session {} not found", id))?;
        session.screen.resize(cols as usize, rows as usize);
        session.cols = cols;
        session.rows = rows;
        Ok(())
    }
}

impl Default for VttyManager {
    fn default() -> Self {
        Self::new()
    }
}
