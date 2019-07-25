use termion::{*, event::Key};

use std::io::Write;
use std::cmp;
use std::collections::BTreeMap;

/// The different types a Viewport can be, and their associated data.
pub enum ViewportData {
    Buffer(scribe::Buffer),
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
                        self.starting_visible_line -= self.starting_visible_line - buffer.cursor.line;
                    }

                    // Update the cursor: are we out of view horizontally and need to scroll?
                    if buffer.cursor.offset >= self.starting_visible_column {
                        if buffer.cursor.offset - self.starting_visible_column > self.size.0 - 5 - self.origin.0 as usize { // If buffer's cursor is beyond the visible columns
                            self.starting_visible_column += buffer.cursor.offset - (self.starting_visible_column + (self.size.0 - 5 - self.origin.0 as usize)); // Set visible columns to show at least that column
                        }
                    } else { // We need to scroll left, if the cursor is to the left of the minimum visible line
                        self.starting_visible_column -= self.starting_visible_column - buffer.cursor.offset;
                    }
                }

                // Render the lines from the text
                // TODO: need a much more efficient way of rendering the lines without constructing all of them
                // write text line by line with line numbers
                for (i, mut l) in scribe::util::LineIterator::new(&buffer.data()).skip(self.starting_visible_line).take(self.size.1 - 1) {
                    if self.starting_visible_column + 1 > l.len() { // (+ 1 to prevent subtraction overflow when doing - 1 on l.len())
                        continue; // We don't want to render an empty line (nor index one!)
                    } else {
                        // The line fits within view, so we need to trim it down based on how far we've scrolled right
                        l = &l[self.starting_visible_column..]; // We know the line must begin at the first visible column...
                        if l.len() >= self.size.0 - 5 {
                            l = &l[..cmp::min(l.len()-1, self.size.0 - 5 - 1)]; // Cut either the entire line, or whatever can fit within view
                        }
                    }

                    write!(
                        s,
                        "{} {}{}{:>lcount$}{} {}{}",
                        cursor::Goto(self.origin.0, self.origin.1 + (i - self.starting_visible_line) as u16),
                        color::Fg(color::White),
                        if focused { format!("{}", style::Bold) } else { "".to_owned() }, // Bold the line number
                        i + 1,
                        style::NoBold,
                        if focused { format!("{}", color::Fg(color::LightWhite)) } else { "".to_owned() },
                        l,
                        lcount = 3, // The right align space needed for the line number
                    ) // TODO: think about lines that are LESS than the visibility
                    .unwrap();

                    // Horizontal scrolling indicators
                    if self.starting_visible_column > 0 {
                        write!(s, "{}{}{}", cursor::Goto(self.origin.0 + 5, self.origin.1 + (i - self.starting_visible_line) as u16), color::Fg(color::Yellow), "<").unwrap();
                    }
                    if l.len() > self.size.0 - 5 {
                        write!(s, "{}{}{}", cursor::Goto(self.origin.0 + self.size.0 as u16 - 1, self.origin.1 + (i - self.starting_visible_line) as u16), color::Fg(color::Yellow), ">").unwrap();
                    }
                }

                if focused {
                    // Render the cursor
                    write!(
                        s,
                        "{}{}",
                        cursor::Goto(
                            self.origin.0 + 4 + (buffer.cursor.position.offset - self.starting_visible_column) as u16 + 1,
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
                let buf = v.get_buffer().unwrap();
                let mut t = buf.file_name().unwrap_or("Untitled".to_owned());
                if buf.modified() {
                    t.insert(0, '*');
                }
                t
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

    pub fn new_viewport(&mut self, data: ViewportData) {
        self.viewports.push(Viewport {
            origin: (self.origin.0 + 1, self.origin.1 + 1),
            size: (self.size.0 - 1, self.size.1 - 2),
            data,
            starting_visible_line: 0,
            starting_visible_column: 0,
        });
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
