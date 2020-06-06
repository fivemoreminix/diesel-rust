//! The high-level abstraction over rendering ANSI text on the terminal.
//! This abstraction is backend-specific, thus there is no such 'backend abstraction'.
//! Rendering graphics is done using high-level functions, that are, by themselves,
//! unrelated to the backend at hand.

// use termion::{color, cursor};
use crossterm::*;
use vek::*;
use std::io::Write;
use lazy_static::*;

static DEFAULT_FG: Fg = Fg(Color::White);
static DEFAULT_BG: Bg = Bg(Color::Black);

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Fg(pub Color);
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Bg(pub Color);

// impl std::fmt::Display for Fg {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         match self.0 {
//             Color::AnsiValue(v) => color::Fg(color::AnsiValue(v)).fmt(f),
//             // Color::RGB(r,g,b)   => color::Fg(color::Rgb(r,g,b)).fmt(f),

//             Color::White   => color::Fg(color::White).fmt(f),
//             Color::Black   => color::Fg(color::Black).fmt(f),
//             Color::Blue    => color::Fg(color::Blue).fmt(f),
//             Color::Cyan    => color::Fg(color::Cyan).fmt(f),
//             Color::Green   => color::Fg(color::Green).fmt(f),
//             Color::Magenta => color::Fg(color::Magenta).fmt(f),
//             Color::Red     => color::Fg(color::Red).fmt(f),
//             Color::Yellow  => color::Fg(color::Yellow).fmt(f),

//             Color::LightWhite   => color::Fg(color::LightWhite).fmt(f),
//             Color::LightBlack   => color::Fg(color::LightBlack).fmt(f),
//             Color::LightBlue    => color::Fg(color::LightBlue).fmt(f),
//             Color::LightCyan    => color::Fg(color::LightCyan).fmt(f),
//             Color::LightGreen   => color::Fg(color::LightGreen).fmt(f),
//             Color::LightMagenta => color::Fg(color::LightMagenta).fmt(f),
//             Color::LightRed     => color::Fg(color::LightRed).fmt(f),
//             Color::LightYellow  => color::Fg(color::LightYellow).fmt(f),
//         }
//     }
// }

// impl std::fmt::Display for Bg {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         match self.0 {
//             Color::AnsiValue(v) => color::Bg(color::AnsiValue(v)).fmt(f),
//             // Color::RGB(r,g,b)   => color::Bg(color::Rgb(r,g,b)).fmt(f),

//             Color::White   => color::Bg(color::White).fmt(f),
//             Color::Black   => color::Bg(color::Black).fmt(f),
//             Color::Blue    => color::Bg(color::Blue).fmt(f),
//             Color::Cyan    => color::Bg(color::Cyan).fmt(f),
//             Color::Green   => color::Bg(color::Green).fmt(f),
//             Color::Magenta => color::Bg(color::Magenta).fmt(f),
//             Color::Red     => color::Bg(color::Red).fmt(f),
//             Color::Yellow  => color::Bg(color::Yellow).fmt(f),

//             Color::LightWhite   => color::Bg(color::LightWhite).fmt(f),
//             Color::LightBlack   => color::Bg(color::LightBlack).fmt(f),
//             Color::LightBlue    => color::Bg(color::LightBlue).fmt(f),
//             Color::LightCyan    => color::Bg(color::LightCyan).fmt(f),
//             Color::LightGreen   => color::Bg(color::LightGreen).fmt(f),
//             Color::LightMagenta => color::Bg(color::LightMagenta).fmt(f),
//             Color::LightRed     => color::Bg(color::LightRed).fmt(f),
//             Color::LightYellow  => color::Bg(color::LightYellow).fmt(f),
//         }
//     }
// }

#[allow(dead_code)]
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Color {
    // Advanced
    AnsiValue(u8),
    // RGB(u8, u8, u8), // NOTE: use AnsiValue instead
    // Basics
    White,
    Black,
    Blue,
    Cyan,
    Green,
    Magenta,
    Red,
    Yellow,
    // Lights
    LightWhite,
    LightBlack,
    LightBlue,
    LightCyan,
    LightGreen,
    LightMagenta,
    LightRed,
    LightYellow
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Cell(char, Fg, Bg);

impl Default for Cell {
    fn default() -> Cell {
        Cell(' ', DEFAULT_FG, DEFAULT_BG)
    }
}

// Credit: zesterer (Joshua Baretto)
#[derive(Clone)]
struct Grid {
    size: Extent2<usize>,
    cells: Vec<Cell>,
}

impl Grid {
    pub fn new(size: Extent2<usize>) -> Self {
        Self {
            size,
            cells: vec![Cell::default(); size.w * size.h],
        }
    }

    pub fn size(&self) -> Extent2<usize> {
        self.size
    }

    pub fn resize(&mut self, new_size: Extent2<usize>) {
        self.cells.resize(new_size.w * new_size.h, Cell::default());
        self.size = new_size;
    }

    fn idx_of(&self, pos: Vec2<usize>) -> Option<usize> {
        if pos.map2(self.size.into(), |e, sz| e < sz).reduce_and() {
            Some(self.size.w * pos.y + pos.x)
        } else {
            None
        }
    }

    pub fn get(&self, pos: impl Into<Vec2<usize>>) -> Cell {
        match self.idx_of(pos.into()) {
            Some(idx) => self.cells
                .get(idx)
                .copied()
                .unwrap_or_default(),
            None => Cell::default(),
        }
    }

    pub fn get_mut(&mut self, pos: impl Into<Vec2<usize>>) -> &mut Cell {
        match self.idx_of(pos.into()) {
            Some(idx) => self.cells
                .get_mut(idx)
                .unwrap(),
            None => panic!("Unable to get cell mutably: out of bounds"),
        }
    }

    pub fn set(&mut self, pos: impl Into<Vec2<usize>>, cell: Cell) {
        match self.idx_of(pos.into()) {
            Some(idx) => {
                self.cells
                    .get_mut(idx)
                    .map(|c| *c = cell);
            },
            None => {},
        }
    }
}

// /// When we need to access already rendered cells on the terminal, we require a double buffer.
// /// The double buffer represents two things: what has already been rendered to the terminal,
// /// and what we're going to render to the terminal next. This is important for rendering
// /// shadows, where we require the character, background, and foreground of a soon-to-be rendered
// /// character cell.
// pub struct RenderBuffer {
//     /// The size of the buffer should match the dimensions of the terminal.
//     size:  Extent2<usize>,
//     // grids.0 is the 'front', and represents what has already been drawn.
//     // grids.1 is the 'back', and represents the immediate that has not yet been drawn.
//     grids: (Grid, Grid),
//     fg:    Fg,
//     bg:    Bg,
// }

// impl RenderBuffer {
//     #[inline]
//     pub fn new(size: (usize, usize)) -> RenderBuffer {
//         let size = Extent2::from(size);
//         let grid = Grid::new(size);
//         RenderBuffer { size, grids: (grid.clone(), grid), fg: DEFAULT_FG, bg: DEFAULT_BG }
//     }

//     /// Truncate cells or append new blank cells to the buffer to fit
//     /// within the bounds of the given new size.
//     /// 
//     /// The RenderBuffer is automatically resized when needed by the `render` function.
//     pub fn resize(&mut self, new_size: Extent2<usize>) {
//         self.grids.0.resize(new_size);
//         self.grids.1.resize(new_size);
//         self.size = new_size;
//     }

//     pub fn auto_resize(&mut self) {
//         let term_size = terminal::size().expect("Could not get terminal size to auto-resize the RenderBuffer");
//         let term_size = Extent2::from((term_size.0 as usize, term_size.1 as usize));
//         if self.size != term_size {
//             self.resize(term_size);
//         }
//     }

//     pub fn set_fg(&mut self, fg: Color) {
//         self.fg = Fg(fg);
//     }

//     pub fn set_bg(&mut self, bg: Color) {
//         self.bg = Bg(bg);
//     }

//     #[inline(always)]
//     pub fn set_cell(&mut self, pos: impl Into<Vec2<usize>>, ch: char) {
//         self.grids.1.set(pos, Cell(ch, self.fg, self.bg))
//     }

//     pub fn draw(&mut self, origin: (usize, usize), draw: Draw) {
//         match draw {
//             Draw::Text(s) => for (i, c) in s.chars().enumerate() {
//                 self.set_cell((origin.0 + i, origin.1), c);
//             },
//             Draw::Rect(w, h) => for x in 0..w {
//                 for y in 0..h {
//                     self.set_cell((origin.0 + x, origin.1 + y), ' ');
//                 }
//             },
//             Draw::BeamRect(w, h) => for y in 0..h {
//                 if y == 0 { // Top row
//                     self.set_cell((origin.0, origin.1), '┌');
//                     for x in 1..w-1 {
//                         self.set_cell((origin.0 + x, origin.1), '─')
//                     }
//                     self.set_cell((origin.0 + (w-1), origin.1 + y), '┐');
//                 } else if y == h - 1 { // Bottom row
//                     self.set_cell((origin.0, origin.1 + y), '└');
//                     for x in 1..w-1 {
//                         self.set_cell((origin.0 + x, origin.1 + y), '─')
//                     }
//                     self.set_cell((origin.0 + (w-1), origin.1 + y), '┘');
//                 } else { // Everything inbetween
//                     self.set_cell((origin.0, origin.1 + y), '│');
//                     self.set_cell((origin.0 + (w-1), origin.1 + y), '│');
//                 }
//             },
//         }
//     }

//     pub fn render_ansi(&mut self) -> String {
//         let mut out = String::new();
        
//         // Instead of zero, we want a completely incorrect value so we set the cursor on first column encountered.
//         let mut last_pos = Vec2::one();
//         let mut last_fg = DEFAULT_FG;
//         let mut last_bg = DEFAULT_BG;

//         for row in 0..self.size.h {
//             for col in 0..self.size.w {
//                 let (front, back) = (self.grids.0.get_mut((col, row)), self.grids.1.get((col, row)));

//                 if *front != back {
//                     if last_pos != Vec2::new(col.saturating_sub(1), row) { // If this cell didn't follow immediately after the last (cursor optimization)
//                         out.push_str(&format!("{}", cursor::Goto(col as u16 + 1, row as u16 + 1)));
//                     }

//                     let Cell(c, fg, bg) = back;
                    
//                     // Color and attributes optimizations. We don't want to write
//                     // an ANSI color value for every character we draw. So we do this to
//                     // minimize the number of ANSI escape sequences we generate.
//                     if last_fg != fg {
//                         out.push_str(&format!("{}", fg));
//                         last_fg = fg;
//                     }
//                     if last_bg != bg {
//                         out.push_str(&format!("{}", bg));
//                         last_bg = bg;
//                     }
//                     out.push(c); // Write the character

//                     *front = back; // Copy cells from the current buffer to the other

//                     last_pos = Vec2::new(col, row); // Update last position
//                 }
//             }
//         }

//         // dbg!(&out);
//         out
//     }

//     pub fn render(&mut self) {
//         let stdout = std::io::stdout();
//         let mut handle = stdout.lock();

//         handle.write_all(self.render_ansi().as_bytes()).unwrap();
//         handle.flush().unwrap();
//     }
// }

/// Different drawing modes for creating shapes and text on the terminal.
pub enum Draw<'a> {
    Text(&'a str),
    Rect(usize, usize),
    BeamRect(usize, usize),
}
