//! The high-level abstraction over rendering ANSI text on the terminal.
//! This abstraction is backend-specific, thus there is no such 'backend abstraction'.
//! Rendering graphics is done using high-level functions, that are, by themselves,
//! unrelated to the backend at hand.

use termion::color;
use std::io::Write;
use vek::vec::repr_simd::extent2::Extent2;

static DEFAULT_FG: Fg = Fg(Color::White);
static DEFAULT_BG: Bg = Bg(Color::Black);
// static DEFAULT_CELL_ATTR: CellAttr = CellAttr { fg: Fg(Color::White), bg: Bg(Color::Black) };
static DEFAULT_CELL: Cell = Cell { ch: ' ', fg: Fg(Color::White), bg: Bg(Color::Black) };

#[derive(Debug, Copy, Clone)]
pub struct Fg(pub Color);
#[derive(Debug, Copy, Clone)]
pub struct Bg(pub Color);

impl std::fmt::Display for Fg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0 {
            Color::White => color::Fg(color::White).fmt(f),
            Color::Black => color::Fg(color::Black).fmt(f),
            Color::LightWhite => color::Fg(color::LightWhite).fmt(f),
            Color::Blue => color::Fg(color::Blue).fmt(f),
        }
    }
}

impl std::fmt::Display for Bg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0 {
            Color::White => color::Bg(color::White).fmt(f),
            Color::Black => color::Bg(color::Black).fmt(f),
            Color::LightWhite => color::Bg(color::LightWhite).fmt(f),
            Color::Blue => color::Bg(color::Blue).fmt(f),
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub enum Color {
    White,
    Black,
    LightWhite,
    Blue,
}

// #[derive(Debug, Clone)]
// pub struct CellAttr {
//     pub fg: Fg,
//     pub bg: Bg,
// }

#[derive(Debug, Clone)]
pub struct Cell {
    ch: char,
    fg: Fg,
    bg: Bg,
}

/*
theory: attributes are only needed when setting cells and for rendering, after rendering,
cell attributes can be entirely discarded. This may mean those attributes can be stored
within a vector and use a sort of binary search or lookup for the already-in-use attributes,
thus saving memory.
*/

#[derive(Debug)]
pub struct RenderBuffer {
    /// The size of the buffer should match the dimensions of the terminal.
    size: (usize, usize),
    xchg: Vec<bool>, // Columns with changes if true, no changes if false
    ychg: Vec<bool>, // Rows with changes if true, no changes if false
    fg:   Fg,
    bg:   Bg,
    data: Vec<Vec<Cell>>, // NOTE: optimize for at most around (500, 100) cells
}

impl RenderBuffer {
    #[inline]
    pub fn new(size: (usize, usize)) -> RenderBuffer {
        let mut buf = RenderBuffer { size: (0, 0), xchg: Vec::new(), ychg: Vec::new(), fg: DEFAULT_FG, bg: DEFAULT_BG, data: Vec::new() };
        buf.resize(size);
        buf
    }

    /// Truncate cells or append new blank cells to the buffer to fit
    /// within the bounds of the given new size.
    pub fn resize(&mut self, new_size: (usize, usize)) {
        if new_size.0 != self.size.0 { // If width has changed
            for row in self.data.iter_mut() {
                row.resize(new_size.0, DEFAULT_CELL.clone()); // Append or truncate columns
            }
            self.xchg.resize(new_size.0, false);
        }
        if new_size.1 != self.size.1 { // If height has changed
            self.data.resize(new_size.1, vec![DEFAULT_CELL.clone(); new_size.0]); // Append or truncate rows
            self.ychg.resize(new_size.1, false);
        }
        self.size = new_size;
    }

    #[inline]
    pub fn get_cell(&mut self, row: usize, col: usize) -> &Cell {
       &self.data[row][col]
    }

    pub fn set_cell(&mut self, row: usize, col: usize, new: char) {
        self.data[row][col] = Cell { ch: new, fg: self.fg, bg: self.bg }; // TODO: prevent copies and try references?
        self.xchg[col] = true;
        self.ychg[row] = true;
    }

    pub fn set_cells(&mut self, row: usize, starting_col: usize, new: &str) {
        let mut chars = new.chars();
        for (i, col) in self.data[row][starting_col..starting_col+new.len()].iter_mut().enumerate() {
            *col = Cell { ch: chars.next().unwrap(), fg: self.fg, bg: self.bg };
            self.xchg[starting_col+i] = true;
        }
        self.ychg[row] = true;
    }

    pub fn set_fg(&mut self, fg: Color) {
        self.fg = Fg(fg);
    }

    pub fn set_bg(&mut self, bg: Color) {
        self.bg = Bg(bg);
    }

    #[inline]
    fn clear_changes(&mut self) {
        for v in self.xchg.iter_mut() { *v = true; }
        for v in self.ychg.iter_mut() { *v = true; }
    }

    /// Generate ANSI instructions and text from changed cells.
    fn gen_ansi(&self) -> String {
        let mut output = String::new();
        for y in self.ychg.iter().enumerate().filter_map(|(i,&v)| if v == true { Some(i) } else { None }) {
            for (x, cell) in self.xchg.iter().enumerate().filter_map(|(x,&v)| if v == true { Some((x, &self.data[y][x])) } else { None }).peekable() {
                output.push_str(&format!("{}{}{}{}", termion::cursor::Goto((x+1) as u16, (y+1) as u16), cell.fg, cell.bg, cell.ch));
            }
        }
        dbg!(&output);
        output
    }

    pub fn render(&mut self) {
        let stdout = std::io::stdout();
        let mut handle = stdout.lock();

        handle.write_all(self.gen_ansi().as_bytes()).unwrap();
        handle.flush().unwrap();

        self.clear_changes();
    }
}
