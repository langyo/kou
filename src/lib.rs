//! kou — Virtual terminal automation.
//!
//! PTY management + a real VT100/ANSI screen emulator (via `vte`) + screen
//! rendering that can rasterise to PNG with proper fonts or describe the frame
//! to a capable terminal through a graphics protocol (kitty / iTerm2).
//! Extracted from tairitsu's vtty engine as a standalone library.
//!
//! ## Quick Start
//!
//! ```no_run
//! use kou::VttyManager;
//!
//! # async fn run() -> anyhow::Result<()> {
//! let mgr = VttyManager::new();
//! let info = mgr.launch("bash", None, &[], 80, 24, None).await?;
//! mgr.send_text(&info.id, "echo hello\n").await?;
//! let screen = mgr.screenshot(&info.id).await?;
//! println!("{}", screen);
//! # Ok(())
//! # }
//! ```

pub mod font;
pub mod graphics;
pub mod pty;
pub mod read;
pub mod render;
pub mod screen;
pub mod vtty;

pub use font::{FontCache, FontFamily, FontSet};
pub use graphics::GraphicsProtocol;
pub use read::{ReadStyle, read, read_default};
pub use render::{render_graphics, render_png, render_png_supersampled};
pub use screen::Screen;
pub use vtty::{SessionInfo, VttyManager, VttySession, VttySessionId, encode_input, parse_keys};
