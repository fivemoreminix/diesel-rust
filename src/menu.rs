// use termion::{*, input::TermRead, event::Key};
use crossterm::{*, style::Color};

use std::io::Write;

/// A horizontal (x by 1) list of menus. Think 'File  Edit  Selection  View ...'
pub struct MenuBar {
    pub selection_index: usize,
    pub menus: Vec<(String, Menu)>,
}

/// A vertical menu of possible actions, which one could possibly expand a sub-menu.
///
/// These are usually rendered by the MenuBar when a menu item was selected.
pub struct Menu {
    pub children: Vec<(String, MenuAction)>,
}

#[derive(Debug)]
pub enum Action {
    // Hardcoded menus //

    // File
    Close, New, Save, SaveAs, Open,

    // Edit
    Undo, Redo,

    // Help
    About,

    // A script made this action (we need to call it)
    Scripted,
}

pub enum MenuAction {
    Separator,
    Action(Action),
    SubMenu(Menu),
}

fn get_menu_shortcut_from_name(name: &str) -> char {
    let mut chars = name.chars();
    while let Some(c) = chars.next() {
        if c == '_' {
            return chars
                .next()
                .expect("Menu item name had '_' with no following shortcut letter.");
        }
    }
    panic!("Menu item had no shortcut.");
}

impl MenuBar {
    pub fn render<S: Write>(&self, s: &mut S, origin: (u16, u16), h_size: usize, focused: bool) {
        crate::util::draw_rectangle(s, &Color::Grey, origin, (h_size, 1));
        queue!(s, style::SetBackgroundColor(Color::Grey));

        queue!(s, cursor::MoveTo(origin.0 + 1, origin.1));
        for (i, (name, _)) in self.menus.iter().enumerate() {
            let is_help: bool;
            if &name[..] == "_Help" { // This is the help menu, we place it at the far right
                is_help = true;
                queue!(s, cursor::SavePosition, cursor::MoveTo(origin.0 + h_size as u16 - name.len() as u16 - 2, origin.1));
            } else {
                is_help = false;
            }

            let (bg, fg) = if focused && i == self.selection_index { (Color::Black, Color::White) } else { (Color::White, Color::Black) };
            queue!(s, style::SetForegroundColor(fg), style::SetBackgroundColor(bg));
            queue!(s, style::Print(" "));
            { // TODO: comment
                // let mut formatted = String::new();
                let mut chars = name.chars();
                while let Some(c) = chars.next() {
                    if c == '_' {
                        if focused {
                            queue!(s, style::Print(chars.next().unwrap()));
                        } else {
                            queue!(s, style::Print(chars.next().unwrap()));
                        }
                    } else {
                        queue!(s, style::Print(c));
                    }
                }
            }
            queue!(s, style::Print(" "));
            
            if is_help {
                queue!(s, cursor::RestorePosition); // If we skipped to the end to print help, let's go back
            }
        }
    }

    fn get_origin_x_of_menu(&self, idx: usize) -> u16 {
        assert!(!self.menus.is_empty());
        if self.menus[idx].0 == "_Help" { // Annoying, Help is planted on the far right for style
            terminal::size().unwrap().0 - 7
        } else {
            (self.menus.iter().take(idx).map(|(name, _)| name.len()).sum::<usize>() // We have a single space before menus are listed off
            + (idx + 1)) // For spaces before and after names (number of items)
            as u16
        }
    }

    /// Returns a menu index and the origin X offset of the menu, for rendering the menu in the correct position.
    pub fn maybe_handle_key_press(&mut self, key: event::KeyEvent) -> Option<(usize, u16)> {
        use event::KeyCode;
        match key.code {
            KeyCode::Right => if self.selection_index + 1 >= self.menus.len() { self.selection_index = 0; } else { self.selection_index += 1; },
            KeyCode::Left => if self.selection_index as isize - 1 < 0 { self.selection_index = self.menus.len()-1; } else { self.selection_index -= 1; },
            KeyCode::Enter => return Some((self.selection_index, self.get_origin_x_of_menu(self.selection_index))),
            KeyCode::Char(key) => {
                let key = key.to_lowercase().next().unwrap();
                for (i, (c, _)) in self
                    .menus
                    .iter()
                    .map(|(s, m)| (get_menu_shortcut_from_name(s), m))
                    .enumerate()
                {
                    if c.to_lowercase().next().unwrap() == key {
                        self.selection_index = i;
                        return Some((i, self.get_origin_x_of_menu(i)));
                    }
                }
            }
            _ => {},
        }
        None
    }
}

impl Menu {
    pub fn render<S: Write>(&self, s: &mut S, origin: (u16, u16), selection_index: usize) {
        let width = self.get_menu_width();

        // Render background box
        crate::util::draw_rectangle(s, &Color::Grey, origin, (width, self.children.len() + 2));

        // Render box outline
        crate::util::draw_thin_unfilled_rectangle(s, &Color::Black, &Color::Grey, origin, (width, self.children.len() + 2));

        for (i, (name, a)) in self.children.iter().enumerate() {
            // goto, print name ; note the spaces before and after name (padding)
            queue!(s, cursor::MoveTo(origin.0 + 1, origin.1 + 1 + i as u16)); // + 1 makes list appear inside menu bounds
            // Background of a selected item is brighter than others
            let (bg, fg) = if i == selection_index { (Color::Black, Color::Grey) } else { (Color::Grey, Color::Black) };
            queue!(s, style::SetForegroundColor(fg), style::SetBackgroundColor(bg));

            match a {
                MenuAction::Separator => queue!(s, style::Print("─".repeat(width - 2))).unwrap(), // width - 2 is the maximum name length
                _ => {
                    let mut chars = name.chars();
                    while let Some(c) = chars.next() {
                        if c == '_' {
                            queue!(s, style::SetForegroundColor(Color::White), style::Print(chars.next().unwrap()), style::SetForegroundColor(fg));
                        } else {
                            queue!(s, style::Print(c));
                        }
                    }
                    queue!(s, style::Print(" ".repeat(width - 2 - if name.contains('_') { name.len() - 1 } else { name.len() } )));
                }
            }
        }
    }

    /// Take over the current thread and handle the menu's input. This causes recursion when expanding
    /// sub-menus.
    pub fn take_over<S: Write>(&self, s: &mut S, x_offset: u16) -> Option<&Action> {
        use event::{KeyCode, KeyEvent, Event};
        let mut selection_index = 0usize;
        loop {
            self.render(s, (x_offset, 1), selection_index);

            s.flush().unwrap();

            // All of the input code for a graphical menu.
            match event::read().unwrap() {
                Event::Key(KeyEvent { code: KeyCode::Up, .. }) => selection_index = self.previous(selection_index),
                Event::Key(KeyEvent { code: KeyCode::Down, .. }) => selection_index = self.next(selection_index),

                // Activate an action or sub-menu expansion using the enter key.
                Event::Key(KeyEvent { code: KeyCode::Enter, .. }) => match &self.children[selection_index].1 {
                    MenuAction::Separator => unreachable!(),
                    MenuAction::Action(action) => return Some(action),
                    MenuAction::SubMenu(menu) => if let Some(action) = menu.take_over(s, x_offset + self.get_menu_width() as u16) {
                        return Some(action);
                    } // We don't want to close this menu if they exited out of the sub-child one.
                },

                // Activate an action or sub-menu expansion using a shortcut.
                Event::Key(KeyEvent { code: KeyCode::Char(c), .. }) => if let Some(menu_index) = self.maybe_handle_key_press(c) {
                    // Update selection index to the menu action we just pressed
                    selection_index = menu_index;
                    // Redraw with new selection index
                    self.render(s, (x_offset, 1), selection_index);

                    let menu_action = &self.children[menu_index].1;
                    match menu_action {
                        MenuAction::Separator => unreachable!(),
                        MenuAction::Action(action) => return Some(action),
                        MenuAction::SubMenu(menu) => match menu.take_over(s, x_offset + self.get_menu_width() as u16) {
                            Some(action) => return Some(action),
                            _ => {} // We don't want to close the menu... same as above ^
                        }
                    }
                } else {
                    break None; // For now, when you press an unknown key it will close the menu.
                },

                _ => break None,
            }
        }
    }

    fn previous(&self, mut selection_index: usize) -> usize {
        // Perform reverse wrapping
        if selection_index as isize - 1 < 0 { selection_index = self.children.len()-1; } else { selection_index -= 1; }
        if let (_, MenuAction::Separator) = self.children[selection_index] { // Skip separators
            return self.previous(selection_index);
        }
        selection_index
    }

    fn next(&self, mut selection_index: usize) -> usize {
        // Perform forward selection wrapping
        if selection_index + 1 >= self.children.len() { selection_index = 0; } else { selection_index += 1; }
        if let (_, MenuAction::Separator) = self.children[selection_index] { // Skip separators (an infinite loop in rare cases)
            return self.next(selection_index);
        }
        selection_index
    }

    /// Returns the minimum width of the menu, without counting any underscores.
    fn get_menu_width(&self) -> usize {
        2 + self.children.iter().map(|(name, _)| if name.contains('_') { name.len() - 1 } else { name.len() }).max().expect(
            "Empty menu has no width"
        )
    }

    /// Returns `true` if the key press was correctly handled,
    /// or `false` if the key could not be handled (or was not recognized).
    fn maybe_handle_key_press(&self, key: char) -> Option<usize> {
        let key = key.to_lowercase().next().unwrap();
        for (c, menu_index) in self
            .children
            .iter()
            .enumerate()
            .filter_map(|(menu_index, (s, a))| match a { MenuAction::Separator=>None, _=>Some((get_menu_shortcut_from_name(s), menu_index)) }) // Ignore separators, too
        {
            if c.to_lowercase().next().unwrap() == key {
                return Some(menu_index);
            }
        }
        None
    }
}
