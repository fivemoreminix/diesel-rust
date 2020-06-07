use crossterm::{*, style::Color};

fn darken_color(c: Color) -> Color {
    unimplemented!()
}

struct ColorManager {
    fg: Color,
    bg: Color,
}

impl ColorManager {
    pub fn new(fg: Color, bg: Color) -> ColorManager {
        ColorManager { fg, bg }
    }

    pub fn set_fg(&mut self, fg: Color) {
        self.fg = fg;
    }

    pub fn set_bg(&mut self, bg: Color) {
        self.bg = bg;
    }

    pub fn graphics_reset_colors<S: Write>(&self, s: &mut S) {
        queue!(s, style::SetForegroundColor(self.fg), style::SetBackgroundColor(self.bg));
    }
}
