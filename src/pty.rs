//! PTY (pseudo-terminal) management — cross-platform.
//!
//! Unix: forkpty via portable-pty.
//! Windows: ConPTY via portable-pty.

use anyhow::Result;
pub use portable_pty::PtyPair;
use std::io::{Read, Write};

pub struct PtyHandle {
    pub pair: PtyPair,
    pub writer: Box<dyn Write + Send>,
    pub reader: Box<dyn Read + Send>,
}

pub fn open_pty(cols: u16, rows: u16) -> Result<PtyHandle> {
    let pty_system = portable_pty::native_pty_system();
    let pair = pty_system.openpty(portable_pty::PtySize {
        rows,
        cols,
        pixel_width: 0,
        pixel_height: 0,
    })?;

    let writer = pair.master.take_writer()?;
    let reader = pair.master.try_clone_reader()?;

    Ok(PtyHandle {
        pair,
        writer,
        reader,
    })
}
