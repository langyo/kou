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

#[derive(Debug, Clone)]
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
        }
    }

    pub fn resize(&mut self, cols: usize, rows: usize) {
        self.cols = cols;
        self.rows = rows;
        self.cells = vec![Cell::default(); cols * rows];
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

    /// Feed a raw PTY byte stream through the vte parser.
    pub fn feed(&mut self, data: &[u8]) {
        let mut parser = vte::Parser::new();
        let mut perf = Perf { screen: self };
        for &b in data {
            parser.advance(&mut perf, b);
        }
    }

    // ── primitive ops used by the performer ──────────────────────────────

    fn put(&mut self, ch: char) {
        let w = char_width(ch) as usize;
        if w == 0 {
            // Zero-width (control/combining): drop — we don't compose clusters.
            return;
        }
        // A double-wide glyph needs two consecutive cells; wrap first if it
        // would overflow the line (don't split a wide char across lines).
        if self.cursor_col + w > self.cols {
            self.line_wrap();
        }
        // Snapshot the pen before borrowing the cell mutably.
        let (fg, bg, bold, italic, underline) = (
            self.pen_fg,
            self.pen_bg,
            self.pen_bold,
            self.pen_italic,
            self.pen_underline,
        );
        let cell = self.cell_mut(self.cursor_row, self.cursor_col);
        cell.ch = ch;
        // Apply the current pen so multi-character coloured runs render and
        // read correctly (a real terminal's SGR state carries across writes).
        cell.fg = fg;
        cell.bg = bg;
        cell.bold = bold;
        cell.italic = italic;
        cell.underline = underline;
        if w == 2 && self.cursor_col + 1 < self.cols {
            // Mark the continuation cell so the renderer skips it and the
            // cursor lands after both cells.
            let cont = self.cell_mut(self.cursor_row, self.cursor_col + 1);
            cont.wide_cont = true;
            cont.bg = bg;
        }
        self.cursor_col += w;
        if self.cursor_col >= self.cols {
            self.cursor_col = self.cols - 1;
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
        self.cursor_col = 0;
        if self.cursor_row + 1 >= self.rows {
            self.scroll_up(1);
        } else {
            self.cursor_row += 1;
        }
    }

    fn scroll_up(&mut self, n: usize) {
        let n = n.min(self.rows);
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

    fn erase_in_line(&mut self, from_cursor: bool) {
        let start = if from_cursor { self.cursor_col } else { 0 };
        for col in start..self.cols {
            *self.cell_mut(self.cursor_row, col) = Cell::default();
        }
    }

    fn erase_in_display(&mut self, from_cursor: bool) {
        let start_row = if from_cursor {
            self.erase_in_line(true);
            self.cursor_row + 1
        } else {
            0
        };
        for row in start_row..self.rows {
            for col in 0..self.cols {
                *self.cell_mut(row, col) = Cell::default();
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
            b'\r' => self.screen.cursor_col = 0,
            b'\x08' => {
                if self.screen.cursor_col > 0 {
                    self.screen.cursor_col -= 1;
                }
            }
            b'\t' => {
                let next = (self.screen.cursor_col + 8) & !7;
                self.screen.cursor_col = next.min(self.screen.cols.saturating_sub(1));
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
            // CUU/CUD/CUF/CUB — cursor up/down/forward/back.
            'A' => self.screen.cursor_row = self.screen.cursor_row.saturating_sub(n),
            'B' => {
                self.screen.cursor_row =
                    (self.screen.cursor_row + n).min(self.screen.rows.saturating_sub(1))
            }
            'C' => {
                self.screen.cursor_col =
                    (self.screen.cursor_col + n).min(self.screen.cols.saturating_sub(1))
            }
            'D' => self.screen.cursor_col = self.screen.cursor_col.saturating_sub(n),
            // CHA — cursor horizontal absolute (column, 1-based).
            'G' => self.screen.cursor_col = row.min(self.screen.cols.saturating_sub(1)),
            // VPA — vertical position absolute (line, 1-based).
            'd' => self.screen.cursor_row = row.min(self.screen.rows.saturating_sub(1)),
            // CUP / HVP — cursor position (1-based).
            'H' | 'f' => {
                self.screen.cursor_row = row.min(self.screen.rows.saturating_sub(1));
                self.screen.cursor_col = col.min(self.screen.cols.saturating_sub(1));
            }
            // ED — erase in display.
            'J' => match p0.unwrap_or(0) {
                0 => self.screen.erase_in_display(true),
                1 => {
                    // erase from start to cursor — approximate as a full clear;
                    // precise-before-cursor is rarely needed for automation.
                    self.screen.erase_in_display(false);
                }
                _ => self.screen.erase_in_display(false),
            },
            // EL — erase in line.
            'K' => match p0.unwrap_or(0) {
                0 => self.screen.erase_in_line(true),
                1 => self.screen.erase_in_line(false),
                2 => self.screen.erase_in_line(false),
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
fn apply_sgr(screen: &mut Screen, params: &Params) {
    let mut idx = 0;
    let flat: Vec<u16> = params.iter().flat_map(|p| p.iter().copied()).collect();
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
}
