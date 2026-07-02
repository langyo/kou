//! PTY (pseudo-terminal) management — cross-platform.
//!
//! Unix: forkpty/openpty via portable-pty.
//! Windows: ConPTY via portable-pty.

use std::io::{Read, Write};
use std::path::Path;

use anyhow::{Context, Result};
use portable_pty::{Child, CommandBuilder, MasterPty, PtySize};
pub use portable_pty::{CommandBuilder as PtyCommand, PtySize as PtyDimensions};

/// A live pseudo-terminal with its child process attached to the slave end.
pub struct Pty {
    pub writer: Box<dyn Write + Send>,
    pub reader: Box<dyn Read + Send>,
    pub child: Box<dyn Child + Send + Sync>,
    master: Box<dyn MasterPty + Send>,
    /// PID of the spawned child, captured right after `spawn_command`.
    pub pid: Option<u32>,
}

impl Pty {
    /// Resize the terminal in rows/cols.
    pub fn resize(&self, cols: u16, rows: u16) -> Result<()> {
        self.master
            .resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .context("resize pty")
    }
}

/// Open a PTY and spawn `command` (split on whitespace into program + args)
/// attached to its slave end. The slave is closed in the parent so the reader
/// receives EOF when the child exits.
///
/// `env` adds or overrides environment variables on top of the parent's
/// environment; pass an empty slice to inherit unchanged.
pub fn spawn(
    command: &str,
    cwd: Option<&Path>,
    env: &[(&str, &str)],
    cols: u16,
    rows: u16,
) -> Result<Pty> {
    let pty_system = portable_pty::native_pty_system();
    let pair = pty_system
        .openpty(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        })
        .context("open pty")?;

    let mut tokens = command.split_whitespace();
    let program = tokens.next().context("command is empty")?;
    let mut cmd = CommandBuilder::new(program);
    cmd.args(tokens);
    if let Some(dir) = cwd {
        cmd.cwd(dir);
    }
    for (k, v) in env {
        cmd.env(k, v);
    }

    let child = pair
        .slave
        .spawn_command(cmd)
        .with_context(|| format!("spawn `{command}`"))?;
    let pid = child.process_id();
    // Drop the slave in the parent so the reader EOFs on child exit.
    drop(pair.slave);

    let writer = pair.master.take_writer().context("take pty writer")?;
    let reader = pair.master.try_clone_reader().context("clone pty reader")?;

    Ok(Pty {
        writer,
        reader,
        child,
        master: pair.master,
        pid,
    })
}

/// Open a raw PTY pair without spawning anything (kept for low-level use).
pub fn open_pty(cols: u16, rows: u16) -> Result<PtyPair> {
    let pty_system = portable_pty::native_pty_system();
    let pair = pty_system.openpty(PtySize {
        rows,
        cols,
        pixel_width: 0,
        pixel_height: 0,
    })?;
    Ok(PtyPair {
        master: pair.master,
        slave: pair.slave,
    })
}

/// A bare PTY pair (master + slave), no child.
pub struct PtyPair {
    pub master: Box<dyn MasterPty + Send>,
    #[allow(dead_code)]
    pub slave: Box<dyn portable_pty::SlavePty + Send>,
}
