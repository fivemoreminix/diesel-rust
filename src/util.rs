use termion::color::Color;
use termion::event::Key;
use termion::input::TermRead;
use termion::*;

use std::io::{stdin, Write};
use std::path::PathBuf;

// pub fn lines(src: &str) -> Vec<&str> {
//     let mut lines = Vec::new();
//     let mut current_start = 0usize; // Index of src that is the beginning of the current line
//     let mut starting_next_line = true; // Index of src that is the end of the current line

//     let mut chars = src.chars().enumerate();
//     while let Some((i, c)) = chars.next() {
//         if starting_next_line {
//             starting_next_line = false;
//             current_start = i;
//         }
//         if c == '\n' {
//             lines.push(&src[current_start..i-1]); // Add the entire line (excluding the newline character)
//             starting_next_line = true;
//         }
//     }

//     lines
// }

pub fn draw_rectangle<S: Write>(s: &mut S, color: &dyn Color, origin: (u16, u16), size: (usize, usize)) {
    write!(s, "{}", color::Bg(color)).unwrap();
    for l in (0..size.1).map(|i| format!("{}{}", cursor::Goto(origin.0, origin.1 + i as u16), " ".repeat(size.0))) {
        write!(s, "{}", l).unwrap();
    }
}

pub fn draw_thin_unfilled_rectangle<S: Write>(s: &mut S, fg_color: &dyn Color, bg_color: &dyn Color, origin: (u16, u16), size: (usize, usize)) {
    assert!(size.0 >= 2);
    write!(s, "{}{}", color::Fg(fg_color), color::Bg(bg_color)).unwrap();
    for l in 0..size.1 {
        if l == 0 {
            // Top row
            write!(s, "{}┌{}┐", cursor::Goto(origin.0, origin.1), "─".repeat(size.0 - 2)).unwrap();
        } else if l == size.1 - 1 {
            // Bottom row
            write!(s, "{}└{}┘", cursor::Goto(origin.0, origin.1 + l as u16), "─".repeat(size.0 - 2)).unwrap();
        } else {
            // Intermediate row
            write!(s, "{}│{}│", cursor::Goto(origin.0, origin.1 + l as u16), cursor::Goto(origin.0 + size.0 as u16 - 1, origin.1 + l as u16)).unwrap();
        }
    }
}

static ALERT_MIN_WIDTH: u16 = 25;
static ALERT_MIN_HEIGHT: u16 = 5;

/// Will block the thread waiting for an input reply from the user,
/// for a message they receive in a dialog box in the middle of the
/// screen.
pub fn alert<S: Write>(s: &mut S, title: &str, body: &str) {
    let (w, h) = terminal_size().unwrap();

    // Adjusted dimensions to fit the text
    let msg_lines: Vec<String> = {
        let mut v = Vec::new();
        let two_thirds = ((2./3.) * w as f32) as usize;
        for mut l in body.lines().map(|s| s.to_owned()) {
            if l.len() > two_thirds { // If the string is too long,
                // Put newlines to split it, then turn into lines and append those.
                l = textwrap::fill(&l, two_thirds);

                for l in l.lines() {
                    v.push(l.to_owned()); // Add those two or more sub-lines
                }
            } else {
                v.push(l);
            }
        }
        v
    };

    let body_max_len = msg_lines.iter().map(|l| l.len()).max().unwrap();

    // Calculating the dimensions of the dialog based on maximum line lengths and title length.
    let mut alert_w: usize = ALERT_MIN_WIDTH as usize
        + (match std::cmp::max(0, body_max_len as isize - ALERT_MIN_WIDTH as isize) {
            0 => 0,
            val => val + 4, // Add some left and right padding to the body text.
        }) as usize;
    alert_w = std::cmp::max(title.len() + 2, alert_w); // At least fit to title length (+ 2 for padding)

    let alert_h: usize = ALERT_MIN_HEIGHT as usize + msg_lines.len();

    // 'o' as in 'origin'
    let o = (w/2 - alert_w as u16/2, h/2 - alert_h as u16/2); // Character cell of top left of dialog

    // Render a white header square
    draw_rectangle(s, &color::LightWhite, o, (alert_w, 1));

    // Render a grey square from o, to o + (alert_w, alert_h)
    draw_rectangle(s, &color::White, (o.0, o.1 + 1), (alert_w, alert_h - 1));

    write!(
        s,
        "{}{}{}{}{}{}{}", cursor::Goto(w/2 - title.len() as u16/2, o.1), color::Fg(color::Black), color::Bg(color::LightWhite),
        style::Bold, title, style::NoBold,
        color::Bg(color::White), // Reset background ahead of time for the future drawing
    ).unwrap(); // line 1

    // Write the message text
    for (i, l) in msg_lines.iter().enumerate() {
        write!(s, "{}{}", cursor::Goto(w/2 - l.len() as u16/2, o.1 + 2 + i as u16), l).unwrap();
    }
    let msg_lines = msg_lines.len() as u16; // Shadow the old variable with just the number of lines in the message

    // Draw the button
    let button = " OK ";
    write!(s, "{}{}▶{}◀{}", cursor::Goto(w/2 - (button.len() as u16 +2/*for arrows*/)/2, o.1 + 3 + msg_lines),
        color::Bg(color::LightWhite), button, color::Bg(color::White),
    ).unwrap();

    s.flush().unwrap();

    // Get input
    for k in stdin().keys() {
        match k.unwrap() {
            Key::Char('\n') => break,
            _ => {}
        }
    }
}

#[derive(Copy, Clone)]
pub enum InputType {
    /// Just ordinary, unchecked text input.
    Any,
    // /// The text input must be any existant path.
    Path,
    // /// The text input must be a valid path pointing to a directory.
    // Folder,
    // /// The text input must be a valid path pointing to a file.
    // File,
}

static PATH_INPUT_MIN_WIDTH: usize = 28;
static PATH_INPUT_HEIGHT: usize = 6;

/// Will block the thread waiting for string input from the user.
/// Will only accept valid input.
pub fn input<S: Write>(s: &mut S, title: &str, initial_input: String, ty: InputType) -> Option<String> { // NOTE: need parent access to re-render (make render trait?)
    let (w, h) = terminal_size().unwrap();

    let mut entered_text = initial_input;

    let dialog_width = std::cmp::max(title.len() + 2, PATH_INPUT_MIN_WIDTH);
    let o = (w/2 - dialog_width as u16/2, h/2 - PATH_INPUT_HEIGHT as u16/2); // Character cell of top left of dialog

    'mainloop: loop {
        // Render a white header square
        draw_rectangle(s, &color::LightWhite, o, (dialog_width, 1));

        // Render a grey square from o, to o + (alert_w, alert_h)
        draw_rectangle(s, &color::White, (o.0, o.1 + 1), (dialog_width, PATH_INPUT_HEIGHT - 1));

        let button_disabled: bool = match ty {
            InputType::Any => false,
            InputType::Path => !PathBuf::from(&entered_text).exists(),
        };

        // Render a white "input box" square in middle of gray square
        draw_rectangle(s, &color::LightWhite, (o.0 + 1, o.1 + 2), (dialog_width - 2, 1));

        // Render title
        write!(s, "{}{}{}{}{}{}", cursor::Goto(w/2 - title.len() as u16/2, o.1),
            color::Fg(color::Black), color::Bg(color::LightWhite),
            style::Bold, title, style::NoBold,
        ).unwrap(); // line 1

        // Render current entered_text in input box
        write!(s, "{}{}", cursor::Goto(o.0 + 2, o.1 + 2), entered_text).unwrap();

        // Render actions
        write!(s, "{}{}{}", cursor::Goto(o.0 + 1, o.1 + 4), color::Bg(color::White), "Cancel=ESCAPE").unwrap();
        if !button_disabled {
            let ok_button = "OK=RETURN";
            write!(s, "{}{}", cursor::Goto(o.0 + dialog_width as u16 - 1 - ok_button.len() as u16, o.1 + 4), ok_button).unwrap();
        }

        // Set cursor position
        write!(s, "{}{}", cursor::Goto(o.0 + 2 + entered_text.len() as u16, o.1 + 2), cursor::Show).unwrap();

        s.flush().unwrap();

        // Get input
        for k in stdin().keys() {
            match k.unwrap() {
                Key::Char('\n') if !button_disabled => return Some(entered_text),
                Key::Char('\n') => {},
                Key::Esc => break 'mainloop,

                Key::Char(c) => entered_text.push(c),
                Key::Backspace if !entered_text.is_empty() => { entered_text.pop().unwrap(); },
                _ => {}
            }
            continue 'mainloop;
        }
    }

    None
}
