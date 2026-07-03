//! VT100/ANSI terminal screen emulator.
//!
//! Processes a byte stream (PTY output) into a 2D grid of styled cells. The
//! stream is run through the [`vte`](https://crates.io/crates/vte) parser, so
//! real CSI/SGR escape sequences are handled — cursor movement, erase, scroll,
//! and the 16-colour SGR palette the renderer draws with — instead of the
//! placeholder "treat ESC as whitespace" pass from the early stub.

use vte::{Params, Perform};

#[derive(Debug, Clone)]
pub struct Cell {
    pub ch: char,
    pub fg: u8,
    pub bg: u8,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    /// `true` for the right-hand continuation cell of a double-wide (CJK)
    /// glyph. Renderers skip these (the wide char is drawn from the left cell).
    pub wide_cont: bool,
}

impl Default for Cell {
    fn default() -> Self {
        // Terminal convention: the default attribute is white-on-black
        // (fg=7, bg=0), so unstyled text renders visible against the dark
        // background and `read` can treat fg=7/bg=0 as "no annotation".
        Cell {
            ch: '\0',
            fg: 7,
            bg: 0,
            bold: false,
            italic: false,
            underline: false,
            wide_cont: false,
        }
    }
}

/// Display width of `ch` on the terminal grid (1 or 2). CJK / full-width /
/// wide emoji count as 2; control chars as 0.
pub fn char_width(ch: char) -> u16 {
    unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0) as u16
}

pub struct Screen {
    pub cols: usize,
    pub rows: usize,
    pub cells: Vec<Cell>,
    pub cursor_row: usize,
    pub cursor_col: usize,
    pub title: String,
    pub alt_screen: bool,
    /// Current "pen" attributes — applied to every character subsequently
    /// written by `put`, exactly like a real terminal's SGR state.
    pub pen_fg: u8,
    pub pen_bg: u8,
    pub pen_bold: bool,
    pub pen_italic: bool,
    pub pen_underline: bool,
    /// Deferred-wrap flag: set when a write fills the last column. The next
    /// printable character wraps to the next line before being drawn, instead
    /// of overwriting the last cell — matching real terminal behaviour.
    pub wrap_pending: bool,
    /// Lines scrolled off the top of the grid, newest last, capped at
    /// [`SCROLLBACK_MAX`]. Stored as rendered text (no cell attributes) which
    /// is all the MCP "scrollback" tool and `find_text` need.
    pub scrollback: Vec<String>,
    /// In-band graphics protocol image store (kitty APC, iTerm2 OSC 1337,
    /// Sixel DCS). Images are placed at the cursor position in effect when
    /// the escape sequence is received.
    pub image_store: crate::graphics::InlineImageStore,
    pub(crate) kitty_state: crate::graphics::KittyDecodeState,
    parser: vte::Parser,
}

/// Maximum number of off-screen lines retained in the scrollback buffer.
pub const SCROLLBACK_MAX: usize = 1000;

impl Clone for Screen {
    fn clone(&self) -> Self {
        Self {
            cols: self.cols,
            rows: self.rows,
            cells: self.cells.clone(),
            cursor_row: self.cursor_row,
            cursor_col: self.cursor_col,
            title: self.title.clone(),
            alt_screen: self.alt_screen,
            pen_fg: self.pen_fg,
            pen_bg: self.pen_bg,
            pen_bold: self.pen_bold,
            pen_italic: self.pen_italic,
            pen_underline: self.pen_underline,
            wrap_pending: self.wrap_pending,
            scrollback: self.scrollback.clone(),
            image_store: self.image_store.clone(),
            kitty_state: self.kitty_state.clone(),
            parser: vte::Parser::new(),
        }
    }
}

impl std::fmt::Debug for Screen {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Screen")
            .field("cols", &self.cols)
            .field("rows", &self.rows)
            .field("cells", &self.cells)
            .field("cursor_row", &self.cursor_row)
            .field("cursor_col", &self.cursor_col)
            .field("title", &self.title)
            .field("alt_screen", &self.alt_screen)
            .field("pen_fg", &self.pen_fg)
            .field("pen_bg", &self.pen_bg)
            .field("pen_bold", &self.pen_bold)
            .field("pen_italic", &self.pen_italic)
            .field("pen_underline", &self.pen_underline)
            .field("wrap_pending", &self.wrap_pending)
            .field("scrollback_len", &self.scrollback.len())
            .field("images", &self.image_store.placements().len())
            .field("parser", &"<vte::Parser>")
            .finish()
    }
}

impl Screen {
    pub fn new(cols: usize, rows: usize) -> Self {
        Self {
            cols,
            rows,
            cells: vec![Cell::default(); cols * rows],
            cursor_row: 0,
            cursor_col: 0,
            title: String::new(),
            alt_screen: false,
            pen_fg: 7,
            pen_bg: 0,
            pen_bold: false,
            pen_italic: false,
            pen_underline: false,
            wrap_pending: false,
            scrollback: Vec::new(),
            image_store: crate::graphics::InlineImageStore::new(),
            kitty_state: crate::graphics::KittyDecodeState::default(),
            parser: vte::Parser::new(),
        }
    }

    pub fn resize(&mut self, cols: usize, rows: usize) {
        let mut new_cells = vec![Cell::default(); cols * rows];
        let copy_rows = rows.min(self.rows);
        let copy_cols = cols.min(self.cols);
        for r in 0..copy_rows {
            for c in 0..copy_cols {
                new_cells[r * cols + c] = self.cell(r, c).clone();
            }
        }
        // Shrinking the height discards the bottom rows — keep them in the
        // scrollback so nothing is silently lost.
        if rows < self.rows {
            for r in rows..self.rows {
                self.scrollback.push(self.row_text(r));
            }
            while self.scrollback.len() > SCROLLBACK_MAX {
                self.scrollback.remove(0);
            }
        }
        self.cells = new_cells;
        self.cols = cols;
        self.rows = rows;
        self.cursor_row = self.cursor_row.min(rows.saturating_sub(1));
        self.cursor_col = self.cursor_col.min(cols.saturating_sub(1));
    }

    /// Plain-text view of the grid (trailing blanks trimmed per line). Wide
    /// characters contribute one codepoint but two display columns; their
    /// continuation cells are skipped so no spurious space is emitted.
    pub fn text(&self) -> String {
        let mut out = String::with_capacity(self.cols * self.rows);
        for row in 0..self.rows {
            let mut line_end = self.cols;
            while line_end > 0 && self.cell(row, line_end - 1).ch == '\0' {
                line_end -= 1;
            }
            for col in 0..line_end {
                if self.cell(row, col).wide_cont {
                    continue;
                }
                let ch = self.cell(row, col).ch;
                out.push(if ch == '\0' { ' ' } else { ch });
            }
            out.push('\n');
        }
        out.trim_end().to_string()
    }

    fn cell(&self, row: usize, col: usize) -> &Cell {
        &self.cells[row * self.cols + col]
    }

    fn cell_mut(&mut self, row: usize, col: usize) -> &mut Cell {
        &mut self.cells[row * self.cols + col]
    }

    /// Render a single grid row to text: trailing blanks trimmed, wide
    /// continuation cells skipped. Empty when the row is all blanks.
    fn row_text(&self, row: usize) -> String {
        let mut line_end = self.cols;
        while line_end > 0 && self.cell(row, line_end - 1).ch == '\0' {
            line_end -= 1;
        }
        let mut out = String::with_capacity(line_end);
        for col in 0..line_end {
            if self.cell(row, col).wide_cont {
                continue;
            }
            let ch = self.cell(row, col).ch;
            out.push(if ch == '\0' { ' ' } else { ch });
        }
        out.trim_end().to_string()
    }

    /// `true` when any cell on the grid holds a non-blank character.
    pub fn has_output(&self) -> bool {
        for row in 0..self.rows {
            for col in 0..self.cols {
                if self.cell(row, col).ch != '\0' {
                    return true;
                }
            }
        }
        false
    }

    /// Find every `(row, char_index)` where `pattern` occurs in a row's text.
    /// `char_index` is the offset within the row's rendered (non-blank) text,
    /// not the grid column — sufficient for the "did this string appear yet"
    /// polling the MCP `wait` tool does.
    pub fn find_text(&self, pattern: &str) -> Vec<(usize, usize)> {
        if pattern.is_empty() {
            return Vec::new();
        }
        let mut hits = Vec::new();
        for row in 0..self.rows {
            let line = self.row_text(row);
            if let Some(pos) = line.find(pattern) {
                hits.push((row, pos));
            }
        }
        hits
    }

    /// All lines scrolled off the top, joined with newlines (oldest first).
    pub fn scrollback_text(&self) -> String {
        self.scrollback.join("\n")
    }

    /// Scrollback followed by the current screen, joined with a newline.
    pub fn scrollback_with_screen(&self) -> String {
        let sb = self.scrollback_text();
        let screen = self.text();
        if sb.is_empty() {
            screen
        } else if screen.is_empty() {
            sb
        } else {
            format!("{sb}\n{screen}")
        }
    }

    /// Feed a raw PTY byte stream through the vte parser, after extracting
    /// in-band graphics protocols (kitty APC) so images are placed inline.
    pub fn feed(&mut self, data: &[u8]) {
        if self.cols == 0 || self.rows == 0 {
            return;
        }
        // Extract kitty APC graphics BEFORE vte parsing so the cursor position
        // used for placement reflects the state from prior feed() calls.
        let apcs = crate::graphics::extract_kitty_apcs(data);
        for (_, _, control, payload) in &apcs {
            crate::graphics::process_kitty_apc(
                &mut self.kitty_state,
                control,
                payload,
                self.cursor_row,
                self.cursor_col,
                &mut self.image_store,
            );
        }
        let mut parser = std::mem::replace(&mut self.parser, vte::Parser::new());
        let mut perf = Perf { screen: self };
        for &b in data {
            parser.advance(&mut perf, b);
        }
        self.parser = parser;
    }

    // ── primitive ops used by the performer ──────────────────────────────

    fn put(&mut self, ch: char) {
        if self.cols == 0 || self.rows == 0 {
            return;
        }
        let w = char_width(ch) as usize;
        if w == 0 {
            // Zero-width (control/combining): drop — we don't compose clusters.
            return;
        }
        // Deferred wrap: if the previous write filled the last column, advance
        // to the next line now, before drawing this character.
        if self.wrap_pending {
            self.line_wrap();
            self.wrap_pending = false;
        }
        // A double-wide glyph needs two consecutive cells; wrap first if it
        // would overflow the line (don't split a wide char across lines).
        if self.cursor_col + w > self.cols {
            self.line_wrap();
        }
        let row = self.cursor_row;
        let col = self.cursor_col;

        // If we're overwriting either half of an existing wide glyph, blank the
        // other half so no stale wide char leaks through (text/render skip
        // continuation cells, which would otherwise hide the new char).
        if self.cell(row, col).wide_cont && col > 0 {
            self.cell_mut(row, col - 1).ch = '\0';
        }
        if w == 1 && col + 1 < self.cols && self.cell(row, col + 1).wide_cont {
            self.cell_mut(row, col + 1).wide_cont = false;
            self.cell_mut(row, col + 1).ch = '\0';
        }

        // Snapshot the pen before borrowing the cell mutably.
        let (fg, bg, bold, italic, underline) = (
            self.pen_fg,
            self.pen_bg,
            self.pen_bold,
            self.pen_italic,
            self.pen_underline,
        );
        let cell = self.cell_mut(row, col);
        cell.ch = ch;
        cell.fg = fg;
        cell.bg = bg;
        cell.bold = bold;
        cell.italic = italic;
        cell.underline = underline;
        cell.wide_cont = false;
        if w == 2 && col + 1 < self.cols {
            // Mark the continuation cell so the renderer skips it and the
            // cursor lands after both cells.
            let cont = self.cell_mut(row, col + 1);
            cont.ch = '\0';
            cont.wide_cont = true;
            cont.bg = bg;
        }
        self.cursor_col += w;
        if self.cursor_col >= self.cols {
            // Filled the last column: park here and defer the wrap to the next
            // printable char (classic terminal line-wrap semantics).
            self.cursor_col = self.cols - 1;
            self.wrap_pending = true;
        }
    }

    fn line_wrap(&mut self) {
        self.cursor_col = 0;
        if self.cursor_row + 1 >= self.rows {
            self.scroll_up(1);
        } else {
            self.cursor_row += 1;
        }
    }

    fn newline(&mut self) {
        self.wrap_pending = false;
        self.cursor_col = 0;
        if self.cursor_row + 1 >= self.rows {
            self.scroll_up(1);
        } else {
            self.cursor_row += 1;
        }
    }

    fn scroll_up(&mut self, n: usize) {
        if self.rows == 0 || self.cols == 0 {
            return;
        }
        let n = n.min(self.rows);
        // Capture the rows about to scroll off the top so the scrollback
        // buffer retains them (capped to SCROLLBACK_MAX).
        for row in 0..n {
            self.scrollback.push(self.row_text(row));
        }
        while self.scrollback.len() > SCROLLBACK_MAX {
            self.scrollback.remove(0);
        }
        for row in 0..self.rows - n {
            for col in 0..self.cols {
                let src = self.cell(row + n, col).clone();
                *self.cell_mut(row, col) = src;
            }
        }
        for row in self.rows - n..self.rows {
            for col in 0..self.cols {
                *self.cell_mut(row, col) = Cell::default();
            }
        }
    }

    fn erase_in_line_to_end(&mut self) {
        for col in self.cursor_col..self.cols {
            *self.cell_mut(self.cursor_row, col) = Cell::default();
        }
    }

    fn erase_in_line_from_start(&mut self) {
        // EL1: erase from start of line through (and including) the cursor cell.
        for col in 0..=self.cursor_col.min(self.cols.saturating_sub(1)) {
            *self.cell_mut(self.cursor_row, col) = Cell::default();
        }
    }

    fn erase_in_line_all(&mut self) {
        for col in 0..self.cols {
            *self.cell_mut(self.cursor_row, col) = Cell::default();
        }
    }

    fn erase_in_display(&mut self, mode: u16) {
        match mode {
            0 => {
                // ED0: cursor to end of screen.
                self.erase_in_line_to_end();
                for row in self.cursor_row + 1..self.rows {
                    for col in 0..self.cols {
                        *self.cell_mut(row, col) = Cell::default();
                    }
                }
            }
            1 => {
                // ED1: start of screen through cursor.
                for row in 0..self.cursor_row {
                    for col in 0..self.cols {
                        *self.cell_mut(row, col) = Cell::default();
                    }
                }
                self.erase_in_line_from_start();
            }
            _ => {
                // ED2: whole screen.
                for row in 0..self.rows {
                    for col in 0..self.cols {
                        *self.cell_mut(row, col) = Cell::default();
                    }
                }
            }
        }
    }
}

/// `vte::Perform` implementation backed by a [`Screen`]. Handles just the
/// sequences a vtty automation tool realistically needs: printable + line
/// discipline, CR/LF/BS, CSI cursor moves and erase, and the basic SGR
/// (bold/italic/underline + 16-colour fg/bg). Unknown sequences are ignored —
/// the grid stays consistent rather than crashing on a new variant.
struct Perf<'a> {
    screen: &'a mut Screen,
}

impl Perform for Perf<'_> {
    fn print(&mut self, c: char) {
        self.screen.put(c);
    }

    fn execute(&mut self, byte: u8) {
        match byte {
            b'\n' | 0x0b | 0x0c => self.screen.newline(),
            b'\r' => {
                self.screen.cursor_col = 0;
                self.screen.wrap_pending = false;
            }
            b'\x08' => {
                if self.screen.cursor_col > 0 {
                    self.screen.cursor_col -= 1;
                }
                self.screen.wrap_pending = false;
            }
            b'\t' => {
                let next = (self.screen.cursor_col + 8) & !7;
                self.screen.cursor_col = next.min(self.screen.cols.saturating_sub(1));
                self.screen.wrap_pending = false;
            }
            _ => {}
        }
    }

    fn csi_dispatch(&mut self, params: &Params, _intermediates: &[u8], _ignore: bool, byte: char) {
        // vte borrows `params` for the call; lift the numbers we need up front
        // so we never borrow `self.screen` while they're alive. Each param is a
        // sub-param slice; we only care about its first value.
        let mut it = params.iter();
        let p0 = it.next().and_then(|s| s.first()).copied();
        let p1 = it.next().and_then(|s| s.first()).copied();
        let n = p0.unwrap_or(1) as usize;
        let row = p0.unwrap_or(1).saturating_sub(1) as usize;
        let col = p1.unwrap_or(1).saturating_sub(1) as usize;
        match byte {
            // CUU/CUD/CUF/CUB — cursor up/down/forward/back. All clear the
            // deferred-wrap flag (the cursor is now explicitly positioned).
            'A' => {
                self.screen.cursor_row = self.screen.cursor_row.saturating_sub(n);
                self.screen.wrap_pending = false;
            }
            'B' => {
                self.screen.cursor_row =
                    (self.screen.cursor_row + n).min(self.screen.rows.saturating_sub(1));
                self.screen.wrap_pending = false;
            }
            'C' => {
                self.screen.cursor_col =
                    (self.screen.cursor_col + n).min(self.screen.cols.saturating_sub(1));
                self.screen.wrap_pending = false;
            }
            'D' => {
                self.screen.cursor_col = self.screen.cursor_col.saturating_sub(n);
                self.screen.wrap_pending = false;
            }
            // CHA — cursor horizontal absolute (column, 1-based).
            'G' => {
                self.screen.cursor_col = row.min(self.screen.cols.saturating_sub(1));
                self.screen.wrap_pending = false;
            }
            // VPA — vertical position absolute (line, 1-based).
            'd' => {
                self.screen.cursor_row = row.min(self.screen.rows.saturating_sub(1));
                self.screen.wrap_pending = false;
            }
            // CUP / HVP — cursor position (1-based).
            'H' | 'f' => {
                self.screen.cursor_row = row.min(self.screen.rows.saturating_sub(1));
                self.screen.cursor_col = col.min(self.screen.cols.saturating_sub(1));
                self.screen.wrap_pending = false;
            }
            // ED — erase in display.
            'J' => self.screen.erase_in_display(p0.unwrap_or(0)),
            // EL — erase in line.
            'K' => match p0.unwrap_or(0) {
                0 => self.screen.erase_in_line_to_end(),
                1 => self.screen.erase_in_line_from_start(),
                2 => self.screen.erase_in_line_all(),
                _ => {}
            },
            // SU — scroll up.
            'S' => self.screen.scroll_up(n),
            // SGR — graphics rendition.
            'm' => apply_sgr(self.screen, params),
            _ => {}
        }
    }
}

/// Apply a basic SGR parameter stream: styles + the 16-colour palette. The
/// terminal model is a "pen" — attributes set here apply to every character
/// written afterwards, not just the cell under the cursor.
///
/// 256-colour (`38;5;n` / `48;5;n`) and truecolor (`38;2;r;g;b` /
/// `48;2;r;g;b`) sequences are recognised and consumed in full; their index is
/// mapped onto the 16-colour palette when it falls in the ANSI range (so a
/// rendered cell still carries a representative colour), otherwise the pen is
/// left unchanged. This keeps the grid self-consistent instead of mis-parsing
/// the colour payload as spurious bold/italic attributes.
fn apply_sgr(screen: &mut Screen, params: &Params) {
    let flat: Vec<u16> = params.iter().flat_map(|p| p.iter().copied()).collect();
    let mut idx = 0;
    while idx < flat.len() {
        match flat[idx] {
            0 => {
                screen.pen_fg = 7;
                screen.pen_bg = 0;
                screen.pen_bold = false;
                screen.pen_italic = false;
                screen.pen_underline = false;
            }
            1 => screen.pen_bold = true,
            3 => screen.pen_italic = true,
            4 => screen.pen_underline = true,
            22 => screen.pen_bold = false,
            23 => screen.pen_italic = false,
            24 => screen.pen_underline = false,
            30..=37 => screen.pen_fg = (flat[idx] - 30) as u8,
            40..=47 => screen.pen_bg = (flat[idx] - 40) as u8,
            90..=97 => screen.pen_fg = (flat[idx] - 90 + 8) as u8,
            100..=107 => screen.pen_bg = (flat[idx] - 100 + 8) as u8,
            39 => screen.pen_fg = 7, // default fg
            49 => screen.pen_bg = 0, // default bg
            38 | 48 => {
                // Extended colour: `<38|48>;5;n` (256) or `<38|48>;2;r;g;b` (RGB).
                let target_fg = flat[idx] == 38;
                idx += 1;
                if idx >= flat.len() {
                    break;
                }
                match flat[idx] {
                    5 => {
                        // 256-colour: index 0..15 maps to the ANSI palette;
                        // everything else is left as the current pen colour.
                        idx += 1;
                        if idx < flat.len() {
                            let n = flat[idx];
                            if n < 16 {
                                if target_fg {
                                    screen.pen_fg = n as u8;
                                } else {
                                    screen.pen_bg = n as u8;
                                }
                            }
                        }
                    }
                    2 => {
                        // truecolor r;g;b — consume all four (mode + 3 channels).
                        idx += 3;
                    }
                    _ => {}
                }
            }
            _ => {}
        }
        idx += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn feeds_text_into_the_grid() {
        let mut s = Screen::new(10, 3);
        s.feed(b"hello\nworld");
        assert_eq!(s.cell(0, 0).ch, 'h');
        assert_eq!(s.cell(0, 4).ch, 'o');
        assert_eq!(s.cell(1, 0).ch, 'w');
        assert_eq!(s.text(), "hello\nworld");
    }

    #[test]
    fn cursor_position_is_1_based() {
        let mut s = Screen::new(10, 10);
        s.feed(b"\x1b[3;5HX");
        assert_eq!(s.cell(2, 4).ch, 'X');
    }

    #[test]
    fn sgr_paints_the_palette() {
        let mut s = Screen::new(10, 1);
        s.feed(b"\x1b[31mR");
        assert_eq!(s.cell(0, 0).fg, 1); // red
        assert_eq!(s.cell(0, 0).ch, 'R');
    }

    #[test]
    fn erase_in_line_clears_to_end() {
        let mut s = Screen::new(5, 1);
        s.feed(b"hello\x1b[2G\x1b[K");
        assert_eq!(s.cell(0, 0).ch, 'h');
        assert_eq!(s.cell(0, 1).ch, '\0');
        assert_eq!(s.cell(0, 4).ch, '\0');
    }

    #[test]
    fn cjk_chars_are_double_width_on_the_grid() {
        let mut s = Screen::new(12, 1);
        s.feed("ab中".as_bytes());
        // 'a' at col 0, 'b' at col 1, '中' occupies cols 2..4.
        assert_eq!(s.cell(0, 0).ch, 'a');
        assert_eq!(s.cell(0, 1).ch, 'b');
        assert_eq!(s.cell(0, 2).ch, '中');
        assert!(s.cell(0, 3).wide_cont, "continuation cell not marked");
        assert_eq!(s.cursor_col, 4);
    }

    #[test]
    fn wide_char_does_not_split_across_lines() {
        // A 3-col line: place a CJK char that would start at the last column —
        // it must wrap to the next line rather than split.
        let mut s = Screen::new(3, 2);
        s.feed("ab中".as_bytes());
        assert_eq!(s.cell(0, 2).ch, '\0', "col 2 of row 0 should be empty");
        assert_eq!(s.cell(1, 0).ch, '中');
        assert!(s.cell(1, 1).wide_cont);
    }

    #[test]
    fn full_line_wraps_instead_of_overwriting() {
        // A line that fills the last column must wrap to the next line on the
        // following character, not overwrite the last cell.
        let mut s = Screen::new(3, 2);
        s.feed("abcdef".as_bytes());
        assert_eq!(s.cell(0, 0).ch, 'a');
        assert_eq!(s.cell(0, 2).ch, 'c');
        assert_eq!(s.cell(1, 0).ch, 'd');
        assert_eq!(s.cell(1, 2).ch, 'f');
    }

    #[test]
    fn zero_dimension_screen_does_not_panic() {
        let mut s = Screen::new(0, 0);
        s.feed("anything".as_bytes()); // must not index an empty grid
        s.resize(2, 2);
        s.feed("ab".as_bytes());
        assert_eq!(s.cell(0, 0).ch, 'a');
    }

    #[test]
    fn overwriting_a_wide_char_makes_the_new_char_visible() {
        // '中' at cols 2..4, then move onto its continuation cell and write a
        // narrow char — it must not stay hidden behind wide_cont.
        let mut s = Screen::new(4, 1);
        s.feed("ab中".as_bytes());
        s.feed(b"\x1b[1;4Hc");
        // The left half of the old wide char is blanked; 'c' is visible.
        assert_eq!(s.cell(0, 2).ch, '\0');
        assert_eq!(s.cell(0, 3).ch, 'c');
        assert!(!s.cell(0, 3).wide_cont);
    }

    #[test]
    fn sgr_256_colour_does_not_invent_styles() {
        let mut s = Screen::new(4, 1);
        s.feed(b"\x1b[38;5;1mR");
        assert_eq!(s.cell(0, 0).fg, 1); // red
        assert!(!s.cell(0, 0).bold, "256-colour must not set bold");
        assert!(!s.cell(0, 0).italic, "256-colour must not set italic");
    }

    #[test]
    fn partial_escape_across_feed_calls() {
        // If a PTY read splits `\x1b[31mR` into two chunks, the parser must
        // retain its state across feed() boundaries.
        let mut s = Screen::new(10, 3);
        s.feed(b"\x1b[3"); // incomplete CSI
        s.feed(b"1mR");
        assert_eq!(
            s.cell(0, 0).fg,
            1,
            "should parse red SGR from split sequence"
        );
        assert_eq!(s.cell(0, 0).ch, 'R');
    }

    #[test]
    fn sgr_truecolour_is_consumed_without_side_effects() {
        let mut s = Screen::new(4, 1);
        s.feed(b"\x1b[38;2;1;2;3mT\x1b[0mX");
        // The RGB payload must not be read as styles; after reset, X is plain.
        assert!(!s.cell(0, 0).bold);
        assert_eq!(s.cell(0, 1).ch, 'X');
        assert_eq!(s.cell(0, 1).fg, 7);
    }
}
