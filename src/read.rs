//! LLM-friendly screen reading.
//!
//! A model consuming a terminal frame usually gets the raw text — which drops
//! everything that makes a terminal *legible*: column alignment, the spatial
//! "picture" of the layout, and colour. This module produces richer
//! descriptions so a model can actually reason about what is on screen.
//!
//! Three shapes, selected by [`ReadStyle`] (or `KOU_READ_MODE`):
//!
//! - [`ReadStyle::Raw`] — the plain trimmed text (what `Screen::text` gives).
//!   Use this only when a model explicitly asks for raw output.
//! - [`ReadStyle::Boxed`] — the trimmed content wrapped in an ASCII
//!   box-drawing frame, sized to the content's bounding box. Gives the model a
//!   faithful spatial picture of the layout.
//! - [`ReadStyle::Structured`] (the default) — the boxed frame **plus** a
//!   per-line breakdown into styled spans, so colour and emphasis survive:
//!
//!   ```text
//!   ## kou frame 40×9, cursor (row=6,col=27)
//!   ┌─ kou ───────────────────────────┐
//!   │  Name      Status     Notes     │
//!   │  alpha     OK         started   │
//!   │  bravo     WARN       standby   │
//!   │  charlie   ERR        crashed   │
//!   └─────────────────────────────────┘
//!
//!   ## lines (fg/bg = xterm index; styles noted inline)
//!   1: "  Name      Status     Notes     "
//!   2: "  alpha     " fg=green "OK" fg=white "         started   "
//!   3: "  bravo     " fg=yellow "WARN" fg=white "       standby   "
//!   4: "  charlie   " fg=red "ERR" fg=white "        crashed   "
//!   ```
//!
//! Wide (CJK) characters are accounted for by display width, so the box stays
//! rectangular even with mixed scripts.

use crate::screen::{Screen, char_width};

/// How to describe a screen for a reader.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadStyle {
    /// Plain trimmed text only.
    Raw,
    /// Trimmed content inside an ASCII box-drawing frame.
    Boxed,
    /// Boxed frame + per-line styled spans (colour, emphasis). The default.
    Structured,
}

impl ReadStyle {
    /// The style selected via `KOU_READ_MODE`, defaulting to `Structured`.
    pub fn from_env() -> Self {
        match std::env::var("KOU_READ_MODE")
            .ok()
            .map(|s| s.trim().to_ascii_lowercase())
            .as_deref()
        {
            Some("raw") => ReadStyle::Raw,
            Some("boxed") | Some("box") => ReadStyle::Boxed,
            _ => ReadStyle::Structured,
        }
    }
}

/// Describe `screen` according to `style`.
pub fn read(screen: &Screen, style: ReadStyle) -> String {
    match style {
        ReadStyle::Raw => screen.text(),
        ReadStyle::Boxed => boxed(screen),
        ReadStyle::Structured => structured(screen),
    }
}

/// Describe `screen` using the [`ReadStyle`] selected via `KOU_READ_MODE`.
pub fn read_default(screen: &Screen) -> String {
    read(screen, ReadStyle::from_env())
}

// ── xterm colour names ─────────────────────────────────────────────────────

fn colour_name(idx: u8) -> &'static str {
    match idx & 0x0f {
        0 => "black",
        1 => "red",
        2 => "green",
        3 => "yellow",
        4 => "blue",
        5 => "magenta",
        6 => "cyan",
        7 => "white",
        8 => "bright-black",
        9 => "bright-red",
        10 => "bright-green",
        11 => "bright-yellow",
        12 => "bright-blue",
        13 => "bright-magenta",
        14 => "bright-cyan",
        _ => "bright-white",
    }
}

// ── content bounds ─────────────────────────────────────────────────────────

/// The tight (min_row..=max_row, min_col..=max_col) bounding box of non-blank
/// cells, by display column. Returns `None` for an entirely blank screen.
fn content_bounds(screen: &Screen) -> Option<(usize, usize, usize, usize)> {
    let mut min_row = usize::MAX;
    let mut max_row = 0usize;
    let mut min_col = usize::MAX;
    let mut max_col = 0usize;
    let mut any = false;
    for row in 0..screen.rows {
        for col in 0..screen.cols {
            if screen.cells[row * screen.cols + col].ch != '\0' {
                any = true;
                min_row = min_row.min(row);
                max_row = max_row.max(row);
                min_col = min_col.min(col);
                // Display width of this cell (wide chars claim the next col too).
                let w = if screen.cells[row * screen.cols + col].wide_cont {
                    0
                } else {
                    char_width(screen.cells[row * screen.cols + col].ch) as usize
                };
                max_col = max_col.max(col + w.saturating_sub(1));
            }
        }
    }
    if !any {
        return None;
    }
    Some((min_row, max_row, min_col, max_col))
}

/// Build the trimmed, display-width-aware lines within the content bounds.
/// Each line is the raw run of cells (wide continuations skipped) — the caller
/// pads to a fixed display width.
fn trimmed_lines(screen: &Screen, bounds: (usize, usize, usize, usize)) -> Vec<String> {
    let (min_row, max_row, min_col, max_col) = bounds;
    (min_row..=max_row)
        .map(|row| {
            let mut s = String::new();
            for col in min_col..=max_col {
                if col >= screen.cols {
                    break;
                }
                let cell = &screen.cells[row * screen.cols + col];
                if cell.wide_cont {
                    continue;
                }
                s.push(if cell.ch == '\0' { ' ' } else { cell.ch });
            }
            s.trim_end().to_string()
        })
        .collect()
}

/// Display width of a string (each codepoint weighted by `char_width`).
fn disp_width(s: &str) -> usize {
    s.chars().map(|c| char_width(c) as usize).sum()
}

/// Pad `s` with trailing spaces to `width` display columns.
fn pad_to(s: &str, width: usize) -> String {
    let mut out = s.to_string();
    let w = disp_width(s);
    if w < width {
        out.push_str(&" ".repeat(width - w));
    }
    out
}

// ── boxed ──────────────────────────────────────────────────────────────────

const BOX_TITLE: &str = " kou ";

fn boxed(screen: &Screen) -> String {
    match content_bounds(screen) {
        None => "(blank terminal)\n".to_string(),
        Some(bounds) => {
            let (_, _, min_col, max_col) = bounds;
            let inner_w = max_col.saturating_sub(min_col) + 1;
            let lines = trimmed_lines(screen, bounds);
            frame(&lines, inner_w, None)
        }
    }
}

/// Wrap `lines` (already trimmed) in a box-drawing frame of display width
/// `inner_w`, optionally with a header line describing the frame.
fn frame(lines: &[String], inner_w: usize, header: Option<&str>) -> String {
    let title_w = disp_width(BOX_TITLE);
    let cap = if inner_w >= title_w {
        format!(
            "┌{title}{}┐",
            "─".repeat(inner_w - title_w),
            title = BOX_TITLE
        )
    } else {
        format!("┌{}┐", "─".repeat(inner_w))
    };
    let bottom = format!("└{}┘", "─".repeat(inner_w));
    let mut out = String::new();
    if let Some(h) = header {
        out.push_str(h);
        out.push('\n');
    }
    out.push_str(&cap);
    out.push('\n');
    for ln in lines {
        out.push('│');
        out.push_str(&pad_to(ln, inner_w));
        out.push('│');
        out.push('\n');
    }
    out.push_str(&bottom);
    out
}

// ── structured ─────────────────────────────────────────────────────────────

fn structured(screen: &Screen) -> String {
    let header = format!(
        "## kou frame {cols}×{rows}, cursor (row={cr},col={cc}){alt}",
        cols = screen.cols,
        rows = screen.rows,
        cr = screen.cursor_row,
        cc = screen.cursor_col,
        alt = if screen.alt_screen {
            " [alt-screen]"
        } else {
            ""
        }
    );

    let Some(bounds) = content_bounds(screen) else {
        return format!("{header}\n\n(blank terminal)\n");
    };
    let (_, _, min_col, max_col) = bounds;
    let inner_w = max_col.saturating_sub(min_col) + 1;
    let lines = trimmed_lines(screen, bounds);
    let boxed = frame(&lines, inner_w, None);

    let mut out = String::new();
    out.push_str(&header);
    out.push_str("\n\n");
    out.push_str(&boxed);
    out.push_str("\n\n## lines (fg/bg = xterm colour; styles noted inline)\n");

    let (min_row, _, _, _) = bounds;
    for i in 0..lines.len() {
        out.push_str(&format!("{}: ", min_row + i + 1));
        out.push_str(&render_spans(screen, min_row + i, min_col, max_col));
        out.push('\n');
    }
    out
}

/// Render one content line as a sequence of `"text" style…` segments, where the
/// style annotation is emitted only when fg/bg or emphasis changes from the
/// default. Bare text gets no annotation. Consecutive cells with identical
/// attributes coalesce into one quoted run.
fn render_spans(screen: &Screen, row: usize, min_col: usize, max_col: usize) -> String {
    let mut out = String::new();
    let mut buf = String::new();
    // Effective attributes of the current run: `None` means "bare / default".
    let mut run: Option<Attrs> = None;

    let flush = |buf: &mut String, run: &mut Option<Attrs>, out: &mut String| {
        if buf.is_empty() {
            return;
        }
        let text = std::mem::take(buf);
        out.push('"');
        out.push_str(&text);
        out.push('"');
        if let Some(a) = run.take() {
            out.push(' ');
            out.push_str(&a.describe());
        }
        out.push(' ');
    };

    for col in min_col..=max_col {
        if col >= screen.cols {
            break;
        }
        let cell = &screen.cells[row * screen.cols + col];
        if cell.wide_cont {
            continue;
        }
        let attrs = Attrs::of(cell);
        let cell_run = if attrs.is_default() {
            None
        } else {
            Some(attrs)
        };
        // Start a new run only when the effective attributes change.
        if cell_run != run {
            flush(&mut buf, &mut run, &mut out);
            run = cell_run;
        }
        buf.push(if cell.ch == '\0' { ' ' } else { cell.ch });
    }
    flush(&mut buf, &mut run, &mut out);
    out.trim_end().to_string()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct Attrs {
    fg: u8,
    bg: u8,
    bold: bool,
    italic: bool,
    underline: bool,
}

impl Attrs {
    fn of(c: &crate::screen::Cell) -> Self {
        Attrs {
            fg: c.fg,
            bg: c.bg,
            bold: c.bold,
            italic: c.italic,
            underline: c.underline,
        }
    }
    fn is_default(&self) -> bool {
        // Default fg=7 (white) bg=0 (black) with no emphasis is "bare".
        self.fg == 7 && self.bg == 0 && !self.bold && !self.italic && !self.underline
    }
    fn describe(&self) -> String {
        let mut parts = Vec::new();
        if self.fg != 7 {
            parts.push(format!("fg={}", colour_name(self.fg)));
        }
        if self.bg != 0 {
            parts.push(format!("bg={}", colour_name(self.bg)));
        }
        if self.bold {
            parts.push("bold".to_string());
        }
        if self.italic {
            parts.push("italic".to_string());
        }
        if self.underline {
            parts.push("underline".to_string());
        }
        parts.join(";")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn screen_with(frame: &str) -> Screen {
        let cols = frame.lines().map(|l| l.chars().count()).max().unwrap_or(0);
        let rows = frame.lines().count();
        let mut s = Screen::new(cols.max(1), rows.max(1));
        s.feed(frame.as_bytes());
        s
    }

    #[test]
    fn boxed_wraps_content_in_a_frame() {
        let s = screen_with("hello\nworld");
        let out = boxed(&s);
        assert!(out.contains('┌'), "missing top border: {out}");
        assert!(out.contains("hello"));
        assert!(out.contains("world"));
        assert!(out.contains('└'));
    }

    #[test]
    fn structured_includes_header_box_and_spans() {
        let mut s = screen_with("alpha OK\nbravo ERR");
        // Colour the ERR red on the second line.
        s.feed(b"\x1b[2;7H\x1b[31mERR\x1b[0m");
        let out = structured(&s);
        assert!(out.contains("## kou frame"), "missing header: {out}");
        assert!(out.contains("## lines"), "missing spans section: {out}");
        assert!(out.contains("fg=red"), "missing colour annotation: {out}");
    }

    #[test]
    fn raw_matches_screen_text() {
        let s = screen_with("abc");
        assert_eq!(read(&s, ReadStyle::Raw), s.text());
    }

    #[test]
    fn blank_screen_is_handled() {
        let s = Screen::new(10, 3);
        assert_eq!(boxed(&s), "(blank terminal)\n");
        assert!(structured(&s).contains("(blank terminal)"));
    }

    #[test]
    fn disp_width_counts_cjk_as_two() {
        assert_eq!(disp_width("ab中"), 4);
        assert_eq!(pad_to("中", 4), "中  ".to_string());
    }
}
