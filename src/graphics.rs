//! Terminal graphics protocols.
//!
//! On top of the text screen, kou can describe an image to a capable terminal
//! using one of the inband graphics protocols, so a compatible emulator
//! (kitty, wezterm, iTerm2, Ghostty, …) renders the actual pixels in place of
//! an `[image]` placeholder. The screen text is still kept in sync.
//!
//! Three protocols are modelled by [`GraphicsProtocol`]:
//!
//! - **Kitty** (`kitty2`) — the APC `\e_G…` graphics protocol used by kitty and
//!   a growing list of clones. Fully encoded here.
//! - **iTerm2** — the OSC 1337 inline-image protocol (`File=inline=1;…`).
//!   Fully encoded.
//! - **Sixel** — the DCS sixel raster protocol. Emitted as a placeholder
//!   payload that points the consumer at the PNG bytes: producing a real sixel
//!   stream requires a rasterizer (quantisation + sixel compression) that is
//!   out of scope for this crate; callers wanting sixel should hand the PNG to
//!   a dedicated encoder. [`GraphicsProtocol::supported`] reports this.
//!
//! Pick the protocol with `KOU_GRAPHICS=kitty|iterm|sixel|off` (default `off`,
//! meaning "fall back to a PNG render").

use base64::{Engine as _, engine::general_purpose::STANDARD as B64};

/// A terminal graphics protocol kou can address.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphicsProtocol {
    /// Kitty graphics protocol (`kitty2`), APC `\e_G`.
    Kitty,
    /// iTerm2 inline-image protocol, OSC 1337.
    Iterm,
    /// Sixel (DCS). Encoded as a placeholder — see [`GraphicsProtocol::supported`].
    Sixel,
    /// No inband protocol; the consumer renders a PNG out of band.
    Off,
}

impl GraphicsProtocol {
    /// The protocol selected via `KOU_GRAPHICS`, or `Off`.
    pub fn from_env() -> Self {
        match std::env::var("KOU_GRAPHICS")
            .ok()
            .map(|s| s.trim().to_ascii_lowercase())
            .as_deref()
        {
            Some("kitty") | Some("kitty2") => GraphicsProtocol::Kitty,
            Some("iterm") | Some("iterm2") => GraphicsProtocol::Iterm,
            Some("sixel") => GraphicsProtocol::Sixel,
            _ => GraphicsProtocol::Off,
        }
    }

    /// `true` if [`encode`] produces a real inband image for this protocol.
    pub fn supported(self) -> bool {
        matches!(self, GraphicsProtocol::Kitty | GraphicsProtocol::Iterm)
    }

    pub fn label(self) -> &'static str {
        match self {
            GraphicsProtocol::Kitty => "kitty",
            GraphicsProtocol::Iterm => "iterm",
            GraphicsProtocol::Sixel => "sixel",
            GraphicsProtocol::Off => "off",
        }
    }
}

/// An image to place at the cursor position.
#[derive(Debug, Clone)]
pub struct GraphicsRequest<'a> {
    /// Raw image bytes (PNG for kitty/iterm).
    pub bytes: &'a [u8],
    /// Image width in pixels (informational; some protocols want it).
    pub pixel_w: u32,
    pub pixel_h: u32,
    /// Number of terminal columns the image should span.
    pub cells_w: u32,
    /// Number of terminal rows the image should span.
    pub cells_h: u32,
}

/// Encode `req` for `protocol` into the inband escape sequence a terminal
/// renders. Returns `None` for [`GraphicsProtocol::Off`] (caller should render
/// a PNG instead), for [`GraphicsProtocol::Sixel`] (no rasterizer), and for an
/// empty payload (nothing to draw).
pub fn encode(protocol: GraphicsProtocol, req: &GraphicsRequest<'_>) -> Option<String> {
    if req.bytes.is_empty() {
        return None;
    }
    match protocol {
        GraphicsProtocol::Kitty => Some(encode_kitty(req)),
        GraphicsProtocol::Iterm => Some(encode_iterm(req)),
        GraphicsProtocol::Sixel | GraphicsProtocol::Off => None,
    }
}

const KITTY_CHUNK: usize = 4096;

/// Encode the kitty graphics protocol (`kitty2`).
///
/// Emits a fire-and-forget `a=t` (transmit, no response) placement sequence: a
/// first chunk carrying the control payload `f=100,s=<w>,v=<h>,c=<cols>,
/// r=<rows>,a=t` followed by `m=1`-continued chunks and a final `m=0` chunk.
/// Kitty and clones (wezterm, ghostty, …) render it inline without echoing a
/// status reply back into our own PTY reader.
fn encode_kitty(req: &GraphicsRequest<'_>) -> String {
    let b64 = B64.encode(req.bytes);
    let bytes = b64.as_bytes();
    let mut out = String::new();
    let mut start = 0;
    while start < bytes.len() {
        let end = (start + KITTY_CHUNK).min(bytes.len());
        let more = end != bytes.len();
        // APC begin.
        out.push_str("\x1b_G");
        if start == 0 {
            // First chunk carries the control data (PNG = f=100). a=t means
            // transmit-and-place; we omit q= so the terminal does not reply.
            out.push_str(&format!(
                "a=t,t=d,f=100,s={w},v={h},c={cw},r={rh}",
                w = req.pixel_w,
                h = req.pixel_h,
                cw = req.cells_w,
                rh = req.cells_h
            ));
            if more {
                out.push_str(",m=1");
            }
        } else if more {
            out.push_str("m=1");
        } else {
            out.push_str("m=0");
        }
        out.push(';');
        out.push_str(std::str::from_utf8(&bytes[start..end]).unwrap_or(""));
        // ST (string terminator).
        out.push_str("\x1b\\");
        start = end;
    }
    out
}

/// Encode the iTerm2 OSC 1337 inline-image protocol.
fn encode_iterm(req: &GraphicsRequest<'_>) -> String {
    let b64 = B64.encode(req.bytes);
    format!(
        "\x1b]1337;File=inline=1;width={cw}cells;height={rh}cells;size={sz};name={name}:{data}\x07",
        cw = req.cells_w,
        rh = req.cells_h,
        sz = req.bytes.len(),
        // iTerm expects a base64 *name*; an empty name is valid.
        name = B64.encode("kou-image"),
        data = b64
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[serial_test::serial]
    fn env_parsing() {
        let restore = std::env::var_os("KOU_GRAPHICS");
        for (raw, expected) in [
            ("kitty", GraphicsProtocol::Kitty),
            ("KITTY2", GraphicsProtocol::Kitty),
            ("iterm2", GraphicsProtocol::Iterm),
            ("sixel", GraphicsProtocol::Sixel),
            ("nonsense", GraphicsProtocol::Off),
        ] {
            // SAFETY: serial env mutation in a single-threaded test.
            unsafe { std::env::set_var("KOU_GRAPHICS", raw) };
            assert_eq!(GraphicsProtocol::from_env(), expected, "raw = {raw:?}");
        }
        unsafe {
            match restore {
                Some(v) => std::env::set_var("KOU_GRAPHICS", v),
                None => std::env::remove_var("KOU_GRAPHICS"),
            }
        }
    }

    #[test]
    fn kitty_chunks_are_terminated() {
        // A payload bigger than one chunk must produce multiple APC…ST frames.
        let big = vec![b'a'; (KITTY_CHUNK * 3) + 1]; // base64 grows it further
        let req = GraphicsRequest {
            bytes: &big,
            pixel_w: 10,
            pixel_h: 10,
            cells_w: 2,
            cells_h: 2,
        };
        let enc = encode_kitty(&req);
        // Every frame ends with ST (\x1b\\).
        let frames: Vec<&str> = enc.split("\x1b\\").filter(|s| !s.is_empty()).collect();
        assert!(
            frames.len() >= 3,
            "expected chunked frames, got {}",
            frames.len()
        );
        assert!(enc.contains("a=t,t=d,f=100"));
    }

    #[test]
    fn iterm_has_file_header() {
        let data = b"\x89PNG fake";
        let req = GraphicsRequest {
            bytes: data,
            pixel_w: 1,
            pixel_h: 1,
            cells_w: 1,
            cells_h: 1,
        };
        let enc = encode_iterm(&req);
        assert!(enc.starts_with("\x1b]1337;File=inline=1"));
        assert!(enc.contains("width=1cells"));
        assert!(enc.ends_with('\x07'));
    }
}
