// use termion::color::Color;
// use termion::event::Key;
// use termion::input::TermRead;
// use termion::*;

use crossterm::{*, style::Color, event::Event, event::KeyEvent, event::KeyCode};

use std::io::{stdin, Write};
use std::path::PathBuf;

type S = dyn std::io::Write;

pub fn lines(src: &str) -> Vec<&str> {
    if src.is_empty() {
        return vec!("");
    }

    let mut lines = Vec::new();
    let mut current_start = 0usize; // Index of src that is the beginning of the current line
    let mut starting_next_line = true; // Index of src that is the end of the current line

    let mut chars = src.chars().enumerate();
    while let Some((i, c)) = chars.next() {
        if starting_next_line {
            starting_next_line = false;
            current_start = i;
        }
        if c == '\n' {
            lines.push(&src[current_start..i]); // Add the entire line (excluding the newline character)
            starting_next_line = true;
        }
    }

    if src.ends_with('\n') {
        lines.push("");
    } else if !src[current_start..].is_empty() {
        lines.push(&src[current_start..]);
    }

    lines
}

pub fn draw_rectangle<S: Write>(s: &mut S, color: &Color, origin: (u16, u16), size: (usize, usize)) {
    queue!(s, style::SetBackgroundColor(*color));
    for l in 0..size.1 {
        queue!(s, cursor::MoveTo(origin.0, origin.1 + l as u16), style::Print(" ".repeat(size.0)));
    }
}

pub fn draw_thin_unfilled_rectangle<S: Write>(s: &mut S, fg_color: &Color, bg_color: &Color, origin: (u16, u16), size: (usize, usize)) {
    assert!(size.0 >= 2);
    queue!(s, style::SetForegroundColor(*fg_color), style::SetBackgroundColor(*bg_color));
    for l in 0..size.1 {
        if l == 0 {
            // Top row
            queue!(s, cursor::MoveTo(origin.0, origin.1), style::Print(format!("┌{}┐", "─".repeat(size.0 - 2))));
        } else if l == size.1 - 1 {
            // Bottom row
            queue!(s, cursor::MoveTo(origin.0, origin.1 + l as u16), style::Print(format!("└{}┘", "─".repeat(size.0 - 2))));
        } else {
            // Intermediate row
            queue!(s, cursor::MoveTo(origin.0, origin.1 + l as u16), style::Print("│"), cursor::MoveTo(origin.0 + size.0 as u16 - 1, origin.1 + l as u16), style::Print("│"));
        }
    }
}

static ALERT_MIN_WIDTH: u16 = 25;
static ALERT_MIN_HEIGHT: u16 = 5;

/// Will block the thread waiting for an input reply from the user,
/// for a message they receive in a dialog box in the middle of the
/// screen.
pub fn alert<S: Write>(s: &mut S, title: &str, body: &str) {
    let (w, h) = terminal::size().unwrap();

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
    draw_rectangle(s, &Color::White, o, (alert_w, 1));

    // Render a grey square from o, to o + (alert_w, alert_h)
    draw_rectangle(s, &Color::Grey, (o.0, o.1 + 1), (alert_w, alert_h - 1));

    queue!(s,
        cursor::MoveTo(w/2 - title.len() as u16/2, o.1), style::SetForegroundColor(Color::Black), style::SetBackgroundColor(Color::Grey),
        style::PrintStyledContent(style::style(title).attribute(style::Attribute::Bold)), style::SetBackgroundColor(Color::Grey),
    );

    // Write the message text
    for (i, l) in msg_lines.iter().enumerate() {
        queue!(s, cursor::MoveTo(w/2 - l.len() as u16/2, o.1 + 2 + i as u16), style::Print(l));
    }
    let msg_lines = msg_lines.len() as u16; // Shadow the old variable with just the number of lines in the message

    // Draw the button
    let button = " OK ";
    queue!(s,
        cursor::MoveTo(w/2 - (button.len() as u16 + 2) / 2, o.1 + 3 + msg_lines),
        style::PrintStyledContent(style::style(button).on(Color::White)),
    );

    s.flush().unwrap();

    // Get input
    loop {
        match event::read().unwrap() {
            Event::Key(KeyEvent { code: KeyCode::Char('\n'), .. }) => break,
            _ => {},
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
    let (w, h) = terminal::size().unwrap();

    let mut entered_text = initial_input;

    let dialog_width = std::cmp::max(title.len() + 2, PATH_INPUT_MIN_WIDTH);
    let o = (w/2 - dialog_width as u16/2, h/2 - PATH_INPUT_HEIGHT as u16/2); // Character cell of top left of dialog

    'mainloop: loop {
        // Render a white header square
        draw_rectangle(s, &Color::White, o, (dialog_width, 1));

        // Render a grey square from o, to o + (alert_w, alert_h)
        draw_rectangle(s, &Color::Grey, (o.0, o.1 + 1), (dialog_width, PATH_INPUT_HEIGHT - 1));

        let button_disabled: bool = match ty {
            InputType::Any => false,
            InputType::Path => !PathBuf::from(&entered_text).exists(),
        };

        // Render a white "input box" square in middle of gray square
        draw_rectangle(s, &Color::White, (o.0 + 1, o.1 + 2), (dialog_width - 2, 1));

        // Render title
        queue!(s,
            cursor::MoveTo(w/2 - title.len() as u16/2, o.1),
            style::SetForegroundColor(Color::Black), style::SetBackgroundColor(Color::White),
            style::PrintStyledContent(style::style(title).attribute(style::Attribute::Bold)),
        ); // line 1

        // Render current entered_text in input box
        queue!(s,
            cursor::MoveTo(o.0 + 2, o.1 + 2), style::Print(&entered_text)
        );

        // Render actions
        queue!(s,
            cursor::MoveTo(o.0 + 1, o.1 + 4), style::SetBackgroundColor(Color::Grey), style::Print("Cancel=ESCAPE")
        );
        if !button_disabled {
            let ok_button = "OK=RETURN";
            queue!(s, cursor::MoveTo(o.0 + dialog_width as u16 - 1 - ok_button.len() as u16, o.1 + 4), style::Print(ok_button));
        }

        // Set cursor position
        queue!(s, cursor::MoveTo(o.0 + 2 + entered_text.len() as u16, o.1 + 2), cursor::Show);

        s.flush().unwrap();

        // Get input
        loop {
            match event::read().unwrap() {
                Event::Key(KeyEvent { code: KeyCode::Char('\n'), .. }) if !button_disabled => return Some(entered_text),
                Event::Key(KeyEvent { code: KeyCode::Esc, .. }) => break 'mainloop,

                Event::Key(KeyEvent { code: KeyCode::Char(c), .. }) => entered_text.push(c),
                Event::Key(KeyEvent { code: KeyCode::Backspace, .. }) if !entered_text.is_empty() => { entered_text.pop().unwrap(); },
                _ => continue,
            }
        }
    }

    None
}
