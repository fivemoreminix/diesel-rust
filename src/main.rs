// Abandon all hope, ye who enter here:
// When you need a color, set it before writing anything. Never reset colors.

use crossterm::{*, event::{KeyEvent, KeyCode, Event}};

use std::io::{stdin, stdout, Write};
use std::panic;

mod menu;
mod util;
mod viewport;
mod render;

use viewport::{Viewport, ViewportData, ViewportManager};
use render::*;

/// Returns true if the Viewport actually saved the file, or false if the user cancelled.
fn viewport_save_as(viewport: &mut Viewport) -> bool {
    if let Some(file_path_str) = util::input(&mut stdout(), &format!("Save file '{}'", "Untitled"), "./Untitled".to_owned(), util::InputType::Any) {
        let file_path = std::path::PathBuf::from(file_path_str);
        let mut file = std::fs::File::create(&file_path).unwrap(); // Create the file on disk
        file.write_all(viewport.get_buffer().expect("Cannot save a Viewport with no buffer.").data().as_bytes()).expect("Failed to write buffer data into new save file on disk!");
        viewport.data = ViewportData::Buffer(Box::new(scribe::Buffer::from_file(&file_path).unwrap()));
        true
    } else { // If the user inputs no save file path, we do nothing
        false
    }
}

fn main() {
    panic::set_hook(Box::new(|panic_info| util::alert(&mut stdout(), "Panic!", &format!("{}{}", cursor::Show, panic_info))));

    terminal::enable_raw_mode().unwrap();
    execute!(stdout(), cursor::SavePosition, terminal::EnterAlternateScreen);

    let mut screen = stdout();

    let mut size = terminal::size().unwrap();
    
    let mut viewport_manager = ViewportManager {
        origin: (0, 1),
        size: (size.0 as usize, size.1 as usize),
        viewports: Vec::new(),
        focus_index: 0,
    };

    let argv = std::env::args().collect::<Vec<String>>();
    let buf = if argv.len() <= 1 {
        scribe::Buffer::new()
    } else {
        scribe::Buffer::from_file(std::path::Path::new(&argv[1])).unwrap()
    };
    viewport_manager.new_viewport(ViewportData::Buffer(Box::new(buf)));

    // Create and instantiate the default menu bar
    let file = (
        "_File".to_owned(),
        menu::Menu {
            children: vec!(
                ("_New".to_owned(), menu::MenuAction::Action(menu::Action::New)),
                ("_Open".to_owned(), menu::MenuAction::Action(menu::Action::Open)),
                ("".to_owned(), menu::MenuAction::Separator),
                ("_Save".to_owned(), menu::MenuAction::Action(menu::Action::Save)),
                ("Save _as ...".to_owned(), menu::MenuAction::Action(menu::Action::SaveAs)),
                ("".to_owned(), menu::MenuAction::Separator),
                ("_Quit".to_owned(), menu::MenuAction::Action(menu::Action::Close)),
            ),
        },
    );
    let edit = (
        "_Edit".to_owned(),
        menu::Menu {
            children: vec!(
                ("_Undo".to_owned(), menu::MenuAction::Action(menu::Action::Undo)),
                ("_Redo".to_owned(), menu::MenuAction::Action(menu::Action::Redo)),
            ),
        },
    );
    let help = (
        "_Help".to_owned(),
        menu::Menu {
            children: vec!(("_About".to_owned(), menu::MenuAction::Action(menu::Action::About))),
        },
    );
    let mut menu_bar = menu::MenuBar { selection_index: 0, menus: vec!(file, edit, help) };

    let mut in_menu_mode = false;

    loop {
        if viewport_manager.viewports.is_empty() {
            in_menu_mode = true; // If no open editors
        }

        size = terminal::size().unwrap();

        queue!(stdout(), style::SetForegroundColor(style::Color::White), style::SetBackgroundColor(style::Color::Black));
        // write!(screen, "{}{}", color::Bg(color::Black), color::Fg(color::LightWhite)).unwrap();
        // for l in (0..size.1).map(|i| format!("{}{}", cursor::Goto(0, 1 + i as u16), "▒".repeat(size.0 as usize))) {
        //     write!(screen, "{}", l).unwrap();
        // }
        for line in 0..size.1 {
            queue!(stdout(), cursor::MoveTo(0, 1 + line as u16));
            write!(stdout(), "{}", "▒".repeat(size.0 as usize));
        }

        // Set the default terminal colors
        // TODO: We need better coloring infrastructure
        queue!(stdout(), style::SetForegroundColor(style::Color::White), style::SetBackgroundColor(style::Color::Blue)).unwrap();

        queue!(stdout(), cursor::Hide);

        // Update the menu bar
        menu_bar.render(&mut stdout(), (0, 0), size.0 as usize, in_menu_mode);

        // Update all viewports
        viewport_manager.size = (size.0 as usize, size.1 as usize);
        viewport_manager.render(&mut stdout(), !in_menu_mode);

        stdout().flush().unwrap();

        match event::read().unwrap() {
            Event::Key(KeyEvent { code: KeyCode::Esc, .. }) => in_menu_mode = !in_menu_mode,
            Event::Key(KeyEvent { code: KeyCode::Char('q'), modifiers: event::KeyModifiers::CONTROL }) if in_menu_mode => break, // Quit the entire editor TODO: should prompt for save
            Event::Key(KeyEvent { code: KeyCode::Tab, .. }) if in_menu_mode => viewport_manager.next_tab(),
            Event::Key(k) if !in_menu_mode => viewport_manager.handle_key_event(k),
            Event::Key(k) => {
                // High-level action handling
                if let Some((menu_idx, x_offset)) = menu_bar.maybe_handle_key_press(k) {
                    // The menu bar should have set its selection index to the menu at this point, and is re-rendered all while calling 'maybe_handle_key_press'
                    menu_bar.render(&mut screen, (0, 0), size.0 as usize, in_menu_mode);

                    if let Some(action) = menu_bar.menus[menu_idx].1.take_over(&mut screen, x_offset) {
                        use menu::Action::*;
                        match action {
                            Close => if viewport_manager.viewports.is_empty() { break } else { viewport_manager.close_focused_viewport() },

                            New => {
                                viewport_manager.new_viewport(ViewportData::Buffer(Box::new(scribe::Buffer::new()))); // Add viewport
                                viewport_manager.focus_index = viewport_manager.viewports.len()-1; // Set focus to last viewport
                            }
                            Save => {
                                if let Some(viewport) = viewport_manager.get_focused_viewport_mut() {
                                    if let Some(buf) = viewport.get_buffer() {
                                        if buf.modified() { // Only do this code if the buffer is dirty
                                            if buf.file_name().is_some() { // This buffer points to a file on disk
                                                buf.save().unwrap();
                                            } else { // This buffer points to no files on disk
                                                viewport_save_as(viewport);
                                            }
                                        }
                                    }
                                }
                            }
                            SaveAs => {
                                if let Some(viewport) = viewport_manager.get_focused_viewport_mut() {
                                    if viewport.get_buffer().is_some() {
                                        viewport_save_as(viewport);
                                    }
                                }
                            }
                            Open => {
                                if let Some(path) = util::input(&mut screen, "Open file", String::new(), util::InputType::Path) {
                                    let path = std::path::PathBuf::from(path);
                                    if path.is_file() {
                                        let buf = scribe::Buffer::from_file(&path).unwrap();
                                        viewport_manager.new_viewport(ViewportData::Buffer(Box::new(buf)));
                                    } else {
                                        util::alert(&mut screen, "Only accepts files", &format!("You entered {:?}, which is a directory.", path));
                                    }
                                }
                            }

                            About => util::alert(&mut screen, "About QEdit", "QEdit Text Editor\nVersion 0.1\nCopyright © 2019 Luke Wilson.\nLicensed under the MIT License."),
                            _ => util::alert(&mut screen, "Unimplemented action selected", &format!("{:?}", action)),
                        }

                        if !viewport_manager.viewports.is_empty() {
                            in_menu_mode = false; // Go into insert mode automatically when an action has been completed, if there are open viewports.
                        }
                    }
                }
            }
            _ => {}
        }
    }

    execute!(screen, cursor::RestorePosition, terminal::LeaveAlternateScreen, cursor::Show); // Show the cursor so it is not hidden when out of the editor.
}
