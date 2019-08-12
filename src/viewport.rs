use termion::{*, event::Key};

use std::io::Write;
use std::cmp;
use std::collections::BTreeMap;

// Helper functions because float min and max is used in this source file.

#[inline]
fn flt_max(a: f32, b: f32) -> f32 {
    if a < b { b } else { a }
}

#[inline]
fn flt_min(a: f32, b: f32) -> f32 {
    if a > b { b } else { a }
}

/// The different types a Viewport can be, and their associated data.
pub enum ViewportData {
    Buffer(Box<scribe::Buffer>),
    Terminal(String),
}
use ViewportData::*;

/// Like the representation of a terminal within a terminal. Viewports are what
/// make up the editor as the individual windows. They are handled much like a
/// game uses an entity-component-system. The system is the entire editor, the
/// components are the Viewports, and the entities are the ViewportDatas.
///
/// A Viewport's origin and size should not be modified by the Viewport itself.
pub struct Viewport {
    // Common Viewport properties
    pub origin: (u16, u16),
    pub size: (usize, usize),
    pub title: String,

    // What does this Viewport represent?
    pub data: ViewportData,

    // Used for scrolling the text, zero-based.
    pub starting_visible_line: usize,
    pub starting_visible_column: usize,
}

impl Viewport {
    /// Render the Viewport, ready or not.
    pub fn render<S: Write>(&mut self, s: &mut S, focused: bool) {
        match self.data {
            Buffer(ref buffer) => {
                write!(s, "{}{}", color::Bg(color::Blue), color::Fg(color::White)).unwrap();

                // Update cursor and scrolling (cursor rendering happens at the end)
                if focused {
                    // Update the cursor: are we out of view and in need of vertical scrolling?
                    if buffer.cursor.line >= self.starting_visible_line { // Only when it's safe to subtract...
                        if buffer.cursor.line - self.starting_visible_line > self.size.1 - self.origin.1 as usize { // If buffer's cursor is beyond the visible lines
                            self.starting_visible_line += buffer.cursor.line - (self.starting_visible_line + (self.size.1 - self.origin.1 as usize)); // Set visible lines to show at least that line
                        }
                    } else { // We need to scroll up, if the cursor is above the minimum visible line
                        self.starting_visible_line = self.starting_visible_line - (self.starting_visible_line - buffer.cursor.line);
                    }

                    // Update the cursor: are we out of view horizontally and need to scroll?
                    if buffer.cursor.offset >= self.starting_visible_column {
                        if buffer.cursor.offset - self.starting_visible_column > self.size.0 - 5 - self.origin.0 as usize { // If buffer's cursor is beyond the visible columns
                            self.starting_visible_column += buffer.cursor.offset - (self.starting_visible_column + (self.size.0 - 5 - self.origin.0 as usize)); // Set visible columns to show at least that column
                        }
                    } else { // We need to scroll left, if the cursor is to the left of the minimum visible line
                        self.starting_visible_column = self.starting_visible_column - (self.starting_visible_column - buffer.cursor.offset);
                    }
                }

                // Gather the line numbers for the visible portion of the screen.
                let buf_data = buffer.data();
                // let mut lines: Vec<(usize, &str)> = scribe::util::LineIterator::new(&buf_data).skip(self.starting_visible_line).take(self.size.1 - 1).collect();
                let lines: Vec<&str> = crate::util::lines(&buf_data).into_iter().skip(self.starting_visible_line).take(self.size.1 - 1).collect();
                // lines.push((lines.len(), ""));
                // let line_num_digits = lines.iter().map(|(n, _)| n + 1).max().unwrap_or(1).to_string().len();
                let line_num_digits = lines.len().to_string().len(); // Number of digits in the highest line number

                // Render the lines from the text
                for (i, l) in lines.iter().enumerate() {
                    let mut l: String = l.to_string();

                    if self.starting_visible_column > l.len().saturating_sub(1) {
                        continue; // We don't want to render an empty line (nor index one!)
                    } else {
                        // The line fits within view, so we need to trim it down based on how far we've scrolled right
                        let line_length = l.len() - self.starting_visible_column;
                        let chars = l.chars().skip(self.starting_visible_column);
                        l = if line_length >= self.size.0 - 5 {
                            chars.take(cmp::min(l.len()-1, self.size.0 - 5 - 1)).collect() // Cut either the entire line, or whatever can fit within view
                        } else {
                            chars.collect()
                        }
                    }

                    write!(
                        s,
                        "{}{}{}{:>digits$}{} {}{}",
                        cursor::Goto(self.origin.0, self.origin.1 + (i - self.starting_visible_line) as u16),
                        color::Fg(color::White),
                        if focused { format!("{}", style::Bold) } else { "".to_owned() }, // Bold the line number
                        i + 1,
                        style::NoBold,
                        if focused { format!("{}", color::Fg(color::LightWhite)) } else { "".to_owned() },
                        l,
                        digits = line_num_digits, // The right align space needed for the line number
                    )
                    .unwrap();
                }

                if focused {
                    // Render the cursor
                    write!(
                        s,
                        "{}{}",
                        cursor::Goto(
                            self.origin.0 + line_num_digits as u16 + (buffer.cursor.position.offset - self.starting_visible_column) as u16 + 1,
                            self.origin.1 + (buffer.cursor.position.line - self.starting_visible_line) as u16,
                        ),
                        cursor::Show
                    )
                    .unwrap();
                }
            }
            Terminal(ref _lines) => unimplemented!(),
        }
    }

    // TODO: get rid of this later
    pub fn get_buffer(&mut self) -> Option<&mut scribe::Buffer> {
        if let Buffer(buf) = &mut self.data {
            Some(buf)
        } else {
            None
        }
    }

    pub fn vertical_scroll_percent(&self) -> f32 {
        match &self.data {
            Buffer(buffer) => {
                let lines = buffer.line_count();
                // basically a min(1.0, the_expression)
                flt_min(1.0, (self.starting_visible_line + self.size.1 - 1) as f32 / lines as f32)
            }
            Terminal(_) => unimplemented!(),
        }
    }

    /// Insert the given character at the current cursor position or selection.
    pub fn insert(&mut self, ch: char) {
        match self.data {
            Buffer(ref mut buffer) => {
                // lines[self.cursor_pos.1].insert(self.cursor_pos.0, ch);
                // self.cursor_pos.0 += 1;
                buffer.insert(ch.to_string());
                if ch == '\n' {
                    buffer.cursor.move_down();
                }
                buffer.cursor.move_right();
            }
            Terminal(ref _lines) => unimplemented!(),
        }
    }

    /// Delete the character before the current cursor position or selection.
    pub fn backspace(&mut self) {
        match self.data {
            Buffer(ref mut buffer) => {
                // lines[self.cursor_pos.1].remove(self.cursor_pos.0);
                // self.cursor_pos.0 -= 1;
                if buffer.cursor.position.offset > 0 {
                    buffer.cursor.move_to({
                        let mut p = buffer.cursor.position;
                        p.offset -= 1;
                        p
                    });
                } else {
                    // For deleting lines themselves
                    if buffer.cursor.position.line > 0 { // Lines begin counting at zero
                        buffer.cursor.move_up();
                        buffer.cursor.move_to_end_of_line();
                    }
                }

                buffer.delete();
            }
            Terminal(ref _lines) => unimplemented!(),
        }
    }

    /// Delete the character at the current cursor position or selection.
    pub fn delete(&mut self) {
        match self.data {
            Buffer(ref mut buffer) => {
                // lines[self.cursor_pos.1].remove(self.cursor_pos.0);
                // self.cursor_pos.0 -= 1;
                buffer.delete();
            }
            Terminal(ref _lines) => unimplemented!(),
        }
    }

    pub fn save(&mut self) -> std::io::Result<()> {
        if let Some(b) = self.get_buffer() {
            if b.path.is_some() {
                return b.save();
            } else {
                crate::util::alert(&mut std::io::stdout(), "Save File", "Untitled file must be saved.");
            }
        } // Don't do anything if this is a terminal
        Ok(())
    }
}

/// Manages and renders zero or more viewports at any given time. The Viewport Manager
/// tiles viewports, relocating and resizing them so they can fit better within the
/// margins of the screen. This also dispatches events and commands to those viewports,
/// and renders their bounding boxes and titles.
pub struct ViewportManager {
    pub origin: (u16, u16),
    pub size: (usize, usize),
    pub viewports: Vec<Viewport>,
    pub focus_index: usize, // Current index for focused viewport
}

impl ViewportManager {
    pub fn render<S: Write>(&mut self, s: &mut S, has_focus: bool) {
        if self.viewports.is_empty() {
            return; // No need to render nothing.
        }

        // Update proportions of the viewport
        let (v_origin, v_size) = {
            let v = &self.viewports[self.focus_index];
            (v.origin, v.size)
        };

        // Draw the inside of the bounding box
        crate::util::draw_rectangle(s, &color::Blue, (v_origin.0-1, v_origin.1-1), (v_size.0+1, v_size.1+1));
        // Draw the Viewport's 'beam' bounding box
        crate::util::draw_thin_unfilled_rectangle(s, &color::White, &color::Blue, (v_origin.0-1, v_origin.1-1), (v_size.0+1, v_size.1+1));

        {
            let titles: Vec<String> = self.viewports.iter_mut().map(|v| {
            	let mut title = v.title.clone();
                if let Some(buf) = v.get_buffer() {
                	if buf.modified() {
                    	title.insert(0, '*');
                	}
                }
                title
            }).collect();
            let total_len: usize = titles.len() * 3 + titles.iter().map(|t| t.len()).sum::<usize>(); // The number characters all of the titles will take up

            let starting_x: u16 = v_origin.0 + (v_size.0/2 - total_len/2) as u16;
            for (i, t) in titles.iter().enumerate() {
                if i == self.focus_index {
                    write!(s, "{}{}{} {} {}{}", cursor::Goto(starting_x + (i * (t.len() + 3)) as u16, v_origin.1 - 1), color::Fg(color::Blue), color::Bg(color::White), t, color::Fg(color::White), color::Bg(color::Blue)).unwrap();
                } else {
                    write!(s, "{}┤{}├", cursor::Goto(starting_x + (i * (t.len() + 3)) as u16, v_origin.1 - 1), t).unwrap(); // NOTE: skip a char each time
                }
            }
        }

        // Draw the scrollbars
        // Scrollbar height must be between 1 and v_size.1 (height of viewport).
        let scrollbar_height: usize = flt_min((v_size.1 - 1) as f32, flt_max(1.0, v_size.1 as f32 * (v_size.1 as f32 / self.viewports[self.focus_index].get_buffer().unwrap().line_count() as f32))) as usize;
        let scrollbar_v_origin: u16 = v_origin.1 + (f32::from(v_size.1 as u16) * self.viewports[self.focus_index].vertical_scroll_percent()) as u16 - scrollbar_height as u16;
        for i in 0..scrollbar_height {
            write!(s, "{}X", cursor::Goto(v_origin.0 + v_size.0 as u16 - 1, i as u16 + scrollbar_v_origin)).unwrap();
        }
        write!(s, "{}scroll% = {}", cursor::Goto(v_origin.0, v_origin.1 + v_size.1 as u16 - 1), self.viewports[self.focus_index].vertical_scroll_percent() * 100.0).unwrap();

        self.viewports[self.focus_index].render(s, has_focus);
    }

    pub fn handle_key_event(&mut self, key: Key) {
        if self.viewports.is_empty() {
            return; // We cannot handle input without viewports
        }

        let focused_viewport = &mut self.viewports[self.focus_index];
        match key {
            Key::Ctrl('q') => self.close_focused_viewport(),
            Key::Char(c) => focused_viewport.insert(c), // HACKME: not good
            Key::Backspace => focused_viewport.backspace(),
            Key::Delete => focused_viewport.delete(),
            Key::Up => focused_viewport.get_buffer().unwrap().cursor.move_up(),
            Key::Down => focused_viewport.get_buffer().unwrap().cursor.move_down(),
            Key::Right => focused_viewport.get_buffer().unwrap().cursor.move_right(),
            Key::Left => focused_viewport.get_buffer().unwrap().cursor.move_left(),
            _ => crate::util::alert(&mut std::io::stdout(), "Unhandled key event", &format!("{:?}", key)),
        }
    }

    pub fn get_focused_viewport_mut(&mut self) -> Option<&mut Viewport> {
        self.viewports.get_mut(self.focus_index)
    }

    /// Create a new viewport with the given data. Returns the index of the new viewport.
    pub fn new_viewport(&mut self, data: ViewportData) -> usize {
        self.viewports.push(Viewport {
            origin: (self.origin.0 + 1, self.origin.1 + 1),
            size: (self.size.0 - 1, self.size.1 - 2),
            title: match &data {
                ViewportData::Buffer(buf) => buf.file_name().unwrap_or_else(|| "Untitled".to_owned()),
                ViewportData::Terminal(_) => "Terminal".to_owned(),
            },
            data,
            starting_visible_line: 0,
            starting_visible_column: 0,
        });
        self.viewports.len()-1 // Return the index of the created viewport
    }

    pub fn close_focused_viewport(&mut self) {
        if !self.viewports.is_empty() {
            //self.viewports[self.focus_index].save().unwrap(); // TODO: prompt if user wants to save first
            self.viewports.remove(self.focus_index);
            if self.focus_index > 0 { // Only if focus_index is not already zero
                self.focus_index -= 1;
            }
        }
    }

    pub fn next_tab(&mut self) {
        if self.focus_index >= self.viewports.len() - 1 { self.focus_index = 0 } else { self.focus_index += 1 };
    }
}
