//! kou — Virtual terminal automation.
//!
//! PTY management + VT100/ANSI emulation + screen rendering.
//! Extracted from tairitsu's vtty engine as a standalone library.
//!
//! ## Quick Start
//!
//! ```no_run
//! use kou::VttyManager;
//!
//! # async fn run() -> anyhow::Result<()> {
//! let mut mgr = VttyManager::new();
//! let session = mgr.launch("bash", None, 80, 24).await?;
//! mgr.send_text(&session.id, "echo hello\n").await?;
//! let screen = mgr.screenshot(&session.id).await?;
//! println!("{}", screen.text);
//! # Ok(())
//! # }
//! ```

pub mod vtty;
pub mod screen;
pub mod pty;
pub mod render;

pub use vtty::{VttyManager, VttySession, VttySessionId};
