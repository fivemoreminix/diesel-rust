// Abandon all hope, ye who enter here:
// When you need a color, set it before writing anything. Never reset colors.

use termion::raw::IntoRawMode;
use termion::*;
use termion::{event::Key, input::TermRead};

use std::io::{stdin, stdout, Write};
use std::panic;

mod menu;
mod util;
mod viewport;

use viewport::{Viewport, ViewportData, ViewportManager};

fn main() {
    panic::set_hook(Box::new(|panic_info| util::alert(&mut stdout(), "Panic!", &format!("{}{}", cursor::Show, panic_info))));

    let mut screen = screen::AlternateScreen::from(stdout().into_raw_mode().unwrap());
    let mut size = terminal_size().unwrap();

    let argv = std::env::args().collect::<Vec<String>>();
    let buf = if argv.len() <= 1 {
        scribe::Buffer::new()
    } else {
        scribe::Buffer::from_file(std::path::Path::new(&argv[1])).unwrap()
    };
    
    let mut viewport_manager = ViewportManager {
        origin: (1, 2),
        size: (size.0 as usize, size.1 as usize),
        viewports: Vec::new(),
        focus_index: 0,
    };

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
            children: vec!(),
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

        size = terminal_size().unwrap();

        write!(screen, "{}{}", color::Bg(color::Black), color::Fg(color::LightWhite)).unwrap();
        for l in (0..size.1).map(|i| format!("{}{}", cursor::Goto(0, 1 + i as u16), "▒".repeat(size.0 as usize))) {
            write!(screen, "{}", l).unwrap();
        }

        // Set the default terminal colors
        // TODO: We need better coloring infrastructure
        write!(screen, "{}{}", color::Fg(color::White), color::Bg(color::Blue)).unwrap();

        write!(screen, "{}", cursor::Hide).unwrap();

        // Update the menu bar
        menu_bar.render(&mut screen, (1, 1), size.0 as usize, in_menu_mode);

        // Update all viewports
        viewport_manager.size = (size.0 as usize, size.1 as usize);
        viewport_manager.render(&mut screen, !in_menu_mode);

        screen.flush().unwrap();

        if let Some(k) = stdin().keys().next() {
            match k.unwrap() {
                Key::Esc => in_menu_mode = !in_menu_mode,
                Key::Ctrl('q') if in_menu_mode => break, // Quit the entire editor TODO: should prompt for save
                Key::Char('\t') if in_menu_mode => viewport_manager.next_tab(),
                k if !in_menu_mode => viewport_manager.handle_key_event(k),
                k => {
                    // High-level action handling
                    if let Some((menu, x_offset)) = menu_bar.maybe_handle_key_press(k) {
                        if let Some(action) = menu.take_over(&mut screen, x_offset) {
                            use menu::Action::*;
                            match action {
                                Close => if viewport_manager.viewports.is_empty() { break } else { viewport_manager.close_focused_viewport() },

                                New => {
                                    viewport_manager.new_viewport(ViewportData::Buffer(scribe::Buffer::new())); // Add viewport
                                    viewport_manager.focus_index = viewport_manager.viewports.len()-1; // Set focus to last viewport
                                }
                                Save => viewport_manager.get_focused_viewport_mut().unwrap().save().unwrap(),
                                Open => {
                                    if let Some(path) = util::input(&mut screen, "Open file", String::new(), util::InputType::Path) {
                                        util::alert(&mut screen, "You entered", &format!("{:?}", path));
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
            }
            continue;
        }
    }

    write!(screen, "{}", cursor::Show).unwrap(); // Show the cursor so it is not hidden when out of the editor.
}
