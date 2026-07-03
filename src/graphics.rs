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

// ── inband graphics protocol *receiver* ──────────────────────────
//
// The encoding half (above) writes escape sequences that *produce* kitty /
// iTerm2 images. This half is the decoder: it scans a raw PTY byte stream
// for kitty APC sequences, decodes the base64 payload into an
// [`InlineImageStore`], and records a [`Placement`] at the current cursor
// position so the renderer can overlay the image on the pixel canvas.

use std::collections::HashMap;

/// One decoded inline image (stored as raw PNG bytes).
#[derive(Debug, Clone)]
pub struct InlineImage {
    pub id: u32,
    pub data: Vec<u8>,
}

/// A placed image on the screen grid (row/col + dimensions in cells and pixels).
#[derive(Debug, Clone)]
pub struct Placement {
    pub image_id: u32,
    pub row: usize,
    pub col: usize,
    pub pixel_w: u32,
    pub pixel_h: u32,
    pub cells_w: u32,
    pub cells_h: u32,
}

/// Accumulated inline images and their placements.
#[derive(Debug, Clone, Default)]
pub struct InlineImageStore {
    images: HashMap<u32, InlineImage>,
    placements: Vec<Placement>,
}

impl InlineImageStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, img: InlineImage) {
        self.images.insert(img.id, img);
    }

    pub fn place(&mut self, p: Placement) {
        self.placements.push(p);
    }

    pub fn placements(&self) -> &[Placement] {
        &self.placements
    }

    pub fn image(&self, id: u32) -> Option<&InlineImage> {
        self.images.get(&id)
    }

    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.placements.len()
    }
}

/// Partial-state accumulator for chunked kitty APC transfers.
/// Each entry tracks the assembled base64 payload, optional dimension hints,
/// and whether a display (`a=t` / `a=T`) was requested on any chunk.
#[derive(Debug, Clone, Default)]
pub struct KittyDecodeState {
    /// image_id → (accumulated_base64, s=, v=, c=, r=, wants_place)
    pending: HashMap<u32, (Vec<u8>, Option<u32>, Option<u32>, Option<u32>, Option<u32>, bool)>,
}

/// Scan `data` for kitty APC sequences (`\x1b_G…\x1b\\`) and return each as
/// `(start_byte, end_byte, control_string, base64_payload)`.
pub fn extract_kitty_apcs(data: &[u8]) -> Vec<(usize, usize, String, Vec<u8>)> {
    let mut results = Vec::new();
    let mut i = 0;
    while i < data.len().saturating_sub(3) {
        if data[i] == 0x1b && data[i + 1] == b'_' && data[i + 2] == b'G' {
            let start = i;
            let payload_start = i + 3;
            let mut j = payload_start;
            while j < data.len().saturating_sub(1) {
                if data[j] == 0x1b && data[j + 1] == b'\\' {
                    let raw = &data[payload_start..j];
                    let (control, payload) =
                        if let Some(idx) = raw.iter().position(|&b| b == b';') {
                            (
                                String::from_utf8_lossy(&raw[..idx]).to_string(),
                                raw[idx + 1..].to_vec(),
                            )
                        } else {
                            (String::from_utf8_lossy(raw).to_string(), Vec::new())
                        };
                    results.push((start, j + 2, control, payload));
                    i = j + 2;
                    break;
                }
                j += 1;
            }
            if j >= data.len().saturating_sub(1) {
                i += 1;
            }
        } else {
            i += 1;
        }
    }
    results
}

/// Parse a kitty control-string (`key=val,key=val,…`). Returns a flat
/// `Vec<(String,String)>`.
fn parse_control(raw: &str) -> Vec<(String, String)> {
    raw.split(',')
        .filter_map(|pair| {
            let mut kv = pair.splitn(2, '=');
            let k = kv.next()?.trim().to_ascii_lowercase();
            let v = kv.next().unwrap_or("").trim().to_string();
            if k.is_empty() {
                None
            } else {
                Some((k, v))
            }
        })
        .collect()
}

fn ctrl_u32(pairs: &[(String, String)], key: &str) -> Option<u32> {
    pairs
        .iter()
        .find(|(k, _)| k == key)
        .and_then(|(_, v)| v.parse().ok())
}

/// Process one kitty APC `control` + `payload` (base64). Accumulates
/// chunked transfers in `state` and, when a placement is complete,
/// inserts the image into `store` at (`cursor_row`, `cursor_col`).
pub fn process_kitty_apc(
    state: &mut KittyDecodeState,
    control: &str,
    payload: &[u8],
    cursor_row: usize,
    cursor_col: usize,
    store: &mut InlineImageStore,
) {
    let pairs = parse_control(control);
    let image_id = ctrl_u32(&pairs, "i").unwrap_or(0);
    let more = ctrl_u32(&pairs, "m").map(|v| v != 0);
    let is_place = pairs
        .iter()
        .any(|(k, v)| k == "a" && (v == "t" || v == "T"));

    // Accumulate base64 chunks for this image. `wants_place` is sticky:
    // once any chunk requests display (a=t/T), the batch is placed.
    let entry = state
        .pending
        .entry(image_id)
        .or_insert_with(|| (Vec::new(), None, None, None, None, false));
    entry.0.extend_from_slice(payload);
    if entry.1.is_none() {
        entry.1 = ctrl_u32(&pairs, "s");
    }
    if entry.2.is_none() {
        entry.2 = ctrl_u32(&pairs, "v");
    }
    if entry.3.is_none() {
        entry.3 = ctrl_u32(&pairs, "c");
    }
    if entry.4.is_none() {
        entry.4 = ctrl_u32(&pairs, "r");
    }
    if is_place {
        entry.5 = true;
    }

    // Still waiting for more chunks — do not place yet.
    match more {
        Some(true) => return,
        _ => {}
    }

    let (b64, s, v, c, r, wants_place) = match state.pending.remove(&image_id) {
        Some(e) => e,
        None => return,
    };
    if !wants_place {
        return;
    }

    let Ok(data) = B64.decode(&b64) else {
        return;
    };

    let pixel_w = s.unwrap_or(0);
    let pixel_h = v.unwrap_or(0);
    let cells_w = c.unwrap_or(1).max(1);
    let cells_h = r.unwrap_or(1).max(1);

    store.insert(InlineImage {
        id: image_id,
        data,
    });
    store.place(Placement {
        image_id,
        row: cursor_row,
        col: cursor_col,
        pixel_w,
        pixel_h,
        cells_w,
        cells_h,
    });
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

    // ── decoding tests ─────────────────────────────────

    #[test]
    fn decode_kitty_single_frame_creates_placement() {
        let png = minimal_png(4, 4);
        let b64 = B64.encode(&png);
        let seq = format!("\x1b_Ga=t,f=100,c=4,r=2,i=42;{}\x1b\\", b64);
        let mut state = KittyDecodeState::default();
        let mut store = InlineImageStore::new();
        let apcs = extract_kitty_apcs(seq.as_bytes());
        assert_eq!(apcs.len(), 1);
        process_kitty_apc(
            &mut state, &apcs[0].2, &apcs[0].3,
            /* cursor */ 3, 5, &mut store,
        );
        assert_eq!(store.placements().len(), 1);
        let p = &store.placements()[0];
        assert_eq!(p.image_id, 42);
        assert_eq!(p.row, 3);
        assert_eq!(p.col, 5);
        assert_eq!(p.cells_w, 4);
        assert_eq!(p.cells_h, 2);
    }

    #[test]
    fn decode_kitty_multipart_reassembles() {
        let png = minimal_png(2, 2);
        let b64 = B64.encode(&png);
        let mid = b64.len() / 2;
        let chunk0 = format!("\x1b_Ga=t,f=100,c=2,r=2,i=7,m=1;{}", &b64[..mid]);
        // Second chunk carries the rest with i=7 + its own ST terminator.
        let chunk1 = format!("\x1b_Gi=7,m=0;{}\x1b\\", &b64[mid..]);
        let seq = format!("{}\x1b\\{}", chunk0, chunk1);
        let mut state = KittyDecodeState::default();
        let mut store = InlineImageStore::new();
        let apcs = extract_kitty_apcs(seq.as_bytes());
        for (_, _, ctrl, payload) in &apcs {
            process_kitty_apc(&mut state, ctrl, payload, 0, 0, &mut store);
        }
        assert_eq!(store.placements().len(), 1);
        assert_eq!(store.image(7).unwrap().data, png);
    }

    fn minimal_png(w: u32, h: u32) -> Vec<u8> {
        use image::{ImageBuffer, Rgba, ImageEncoder};
        use image::codecs::png::PngEncoder;
        let img: ImageBuffer<Rgba<u8>, Vec<u8>> =
            ImageBuffer::from_pixel(w, h, Rgba([0, 0, 0, 255]));
        let mut buf = Vec::new();
        PngEncoder::new(&mut buf)
            .write_image(&img, w, h, image::ExtendedColorType::Rgba8)
            .unwrap();
        buf
    }
}
