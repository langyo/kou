//! VT100/ANSI terminal screen emulator.
//!
//! Processes byte stream → maintains a 2D grid of cells with attributes.

#[derive(Debug, Clone, Default)]
pub struct Cell {
    pub ch: char,
    pub fg: u8,
    pub bg: u8,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
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
        }
    }

    pub fn resize(&mut self, cols: usize, rows: usize) {
        self.cols = cols;
        self.rows = rows;
        self.cells = vec![Cell::default(); cols * rows];
        self.cursor_row = self.cursor_row.min(rows.saturating_sub(1));
        self.cursor_col = self.cursor_col.min(cols.saturating_sub(1));
    }

    pub fn text(&self) -> String {
        let mut out = String::with_capacity(self.cols * self.rows);
        for row in 0..self.rows {
            let mut line_end = self.cols;
            while line_end > 0 && self.cell(row, line_end - 1).ch == '\0' {
                line_end -= 1;
            }
            for col in 0..line_end {
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

    /// Write a character at cursor position and advance.
    pub fn put_char(&mut self, ch: char) {
        if self.cursor_col >= self.cols {
            self.cursor_col = 0;
            self.cursor_row = (self.cursor_row + 1).min(self.rows - 1);
        }
        let cell = self.cell_mut(self.cursor_row, self.cursor_col);
        cell.ch = ch;
        self.cursor_col += 1;
    }

    /// Process a feed of bytes (ANSI escape sequences + text).
    /// This is a simplified parser — full VT100 emulation will use the `vte` crate.
    pub fn feed(&mut self, data: &[u8]) {
        // TODO: replace with vte-based parser
        let text = String::from_utf8_lossy(data);
        for ch in text.chars() {
            match ch {
                '\n' => {
                    self.cursor_col = 0;
                    self.cursor_row = (self.cursor_row + 1).min(self.rows - 1);
                }
                '\r' => {
                    self.cursor_col = 0;
                }
                '\x08' => {
                    // Backspace
                    if self.cursor_col > 0 {
                        self.cursor_col -= 1;
                    }
                }
                '\x1b' => {
                    // ESC — skip escape sequences for now (simplified)
                    // TODO: use vte crate for proper ANSI parsing
                }
                _ if !ch.is_control() => {
                    self.put_char(ch);
                }
                _ => {}
            }
        }
    }
}
