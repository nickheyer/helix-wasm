use crate::{
    backend::Backend,
    buffer::Cell,
    terminal::Config,
};
use helix_view::graphics::{CursorKind, Rect};
use std::io;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = helixWasm, js_name = writeToTerminal)]
    fn write_to_terminal(data: &str);

    #[wasm_bindgen(js_namespace = helixWasm, js_name = getTerminalSize)]
    fn get_terminal_size() -> Vec<u16>;
}

/// A backend that generates ANSI escape sequences and sends them to
/// JavaScript for rendering in xterm.js.
pub struct WasmBackend {
    buf: String,
    width: u16,
    height: u16,
}

impl WasmBackend {
    pub fn new(width: u16, height: u16) -> Self {
        Self {
            buf: String::with_capacity(4096),
            width,
            height,
        }
    }
}

impl Backend for WasmBackend {
    fn claim(&mut self) -> Result<(), io::Error> {
        // Enter alternate screen
        self.buf.push_str("\x1b[?1049h");
        write_to_terminal(&self.buf);
        self.buf.clear();
        Ok(())
    }

    fn reconfigure(&mut self, _config: Config) -> Result<(), io::Error> {
        Ok(())
    }

    fn restore(&mut self) -> Result<(), io::Error> {
        // Show cursor, exit alternate screen
        self.buf.push_str("\x1b[?25h\x1b[?1049l");
        write_to_terminal(&self.buf);
        self.buf.clear();
        Ok(())
    }

    fn draw<'a, I>(&mut self, content: I) -> Result<(), io::Error>
    where
        I: Iterator<Item = (u16, u16, &'a Cell)>,
    {
        use std::fmt::Write;
        use helix_view::graphics::{Color, Modifier, UnderlineStyle};

        let mut last_x: u16 = u16::MAX;
        let mut last_y: u16 = u16::MAX;
        let mut last_fg = Color::Reset;
        let mut last_bg = Color::Reset;
        let mut last_modifier = Modifier::empty();
        let mut last_underline_style = UnderlineStyle::Reset;
        let mut last_underline_color = Color::Reset;

        for (x, y, cell) in content {
            // Move cursor if not contiguous
            if x != last_x.wrapping_add(1) || y != last_y {
                write!(self.buf, "\x1b[{};{}H", y + 1, x + 1).unwrap();
            }
            last_x = x;
            last_y = y;

            // Emit modifier changes
            if cell.modifier != last_modifier {
                let removed = last_modifier - cell.modifier;
                let added = cell.modifier - last_modifier;

                // For removed modifiers we must reset and re-apply, since
                // there's no individual "off" code for all of them.  The
                // simpler approach: if anything was removed, reset all
                // attributes and re-apply everything.
                if !removed.is_empty() {
                    self.buf.push_str("\x1b[0m");
                    last_fg = Color::Reset;
                    last_bg = Color::Reset;
                    last_underline_style = UnderlineStyle::Reset;
                    last_underline_color = Color::Reset;
                    // Re-apply all current modifiers
                    write_modifiers(&mut self.buf, cell.modifier);
                } else {
                    write_modifiers(&mut self.buf, added);
                }
                last_modifier = cell.modifier;
            }

            // Underline style (curly, dotted, etc.)
            if cell.underline_style != last_underline_style {
                last_underline_style = cell.underline_style;
                match cell.underline_style {
                    UnderlineStyle::Reset => self.buf.push_str("\x1b[24m"),
                    UnderlineStyle::Line => self.buf.push_str("\x1b[4m"),
                    UnderlineStyle::Curl => self.buf.push_str("\x1b[4:3m"),
                    UnderlineStyle::Dotted => self.buf.push_str("\x1b[4:4m"),
                    UnderlineStyle::Dashed => self.buf.push_str("\x1b[4:5m"),
                    UnderlineStyle::DoubleLine => self.buf.push_str("\x1b[21m"),
                }
            }

            // Underline color
            if cell.underline_color != last_underline_color {
                last_underline_color = cell.underline_color;
                write_underline_color(&mut self.buf, cell.underline_color);
            }

            // Set foreground color if changed
            if cell.fg != last_fg {
                last_fg = cell.fg;
                write_color(&mut self.buf, cell.fg, true);
            }

            // Set background color if changed
            if cell.bg != last_bg {
                last_bg = cell.bg;
                write_color(&mut self.buf, cell.bg, false);
            }

            self.buf.push_str(&cell.symbol);
        }

        // Reset attributes
        self.buf.push_str("\x1b[0m");
        Ok(())
    }

    fn hide_cursor(&mut self) -> Result<(), io::Error> {
        self.buf.push_str("\x1b[?25l");
        Ok(())
    }

    fn show_cursor(&mut self, kind: CursorKind) -> Result<(), io::Error> {
        self.buf.push_str("\x1b[?25h");
        match kind {
            CursorKind::Block => self.buf.push_str("\x1b[2 q"),
            CursorKind::Bar => self.buf.push_str("\x1b[6 q"),
            CursorKind::Underline => self.buf.push_str("\x1b[4 q"),
            CursorKind::Hidden => self.buf.push_str("\x1b[?25l"),
        }
        Ok(())
    }

    fn set_cursor(&mut self, x: u16, y: u16) -> Result<(), io::Error> {
        use std::fmt::Write;
        write!(self.buf, "\x1b[{};{}H", y + 1, x + 1).unwrap();
        Ok(())
    }

    fn clear(&mut self) -> Result<(), io::Error> {
        self.buf.push_str("\x1b[2J\x1b[H");
        Ok(())
    }

    fn size(&self) -> Result<Rect, io::Error> {
        let size = get_terminal_size();
        if size.len() >= 2 {
            Ok(Rect::new(0, 0, size[0], size[1]))
        } else {
            Ok(Rect::new(0, 0, self.width, self.height))
        }
    }

    fn flush(&mut self) -> Result<(), io::Error> {
        if !self.buf.is_empty() {
            write_to_terminal(&self.buf);
            self.buf.clear();
        }
        Ok(())
    }

    fn supports_true_color(&self) -> bool {
        true
    }

    fn get_theme_mode(&self) -> Option<helix_view::theme::Mode> {
        None
    }

    fn set_background_color(&mut self, _color: Option<helix_view::theme::Color>) -> io::Result<()> {
        Ok(())
    }
}

fn write_modifiers(buf: &mut String, mods: helix_view::graphics::Modifier) {
    use helix_view::graphics::Modifier;
    if mods.contains(Modifier::BOLD) {
        buf.push_str("\x1b[1m");
    }
    if mods.contains(Modifier::DIM) {
        buf.push_str("\x1b[2m");
    }
    if mods.contains(Modifier::ITALIC) {
        buf.push_str("\x1b[3m");
    }
    if mods.contains(Modifier::SLOW_BLINK) || mods.contains(Modifier::RAPID_BLINK) {
        buf.push_str("\x1b[5m");
    }
    if mods.contains(Modifier::REVERSED) {
        buf.push_str("\x1b[7m");
    }
    if mods.contains(Modifier::HIDDEN) {
        buf.push_str("\x1b[8m");
    }
    if mods.contains(Modifier::CROSSED_OUT) {
        buf.push_str("\x1b[9m");
    }
}

fn write_underline_color(buf: &mut String, color: helix_view::graphics::Color) {
    use helix_view::graphics::Color;
    use std::fmt::Write;
    match color {
        Color::Reset => buf.push_str("\x1b[59m"),
        Color::Rgb(r, g, b) => write!(buf, "\x1b[58:2::{}:{}:{}m", r, g, b).unwrap(),
        Color::Indexed(idx) => write!(buf, "\x1b[58:5:{}m", idx).unwrap(),
        _ => {} // named colors not supported for underline color
    }
}

fn write_color(buf: &mut String, color: helix_view::graphics::Color, foreground: bool) {
    use helix_view::graphics::Color;
    use std::fmt::Write;

    let base = if foreground { 30 } else { 40 };
    match color {
        Color::Reset => write!(buf, "\x1b[{}m", if foreground { 39 } else { 49 }).unwrap(),
        Color::Black => write!(buf, "\x1b[{}m", base).unwrap(),
        Color::Red => write!(buf, "\x1b[{}m", base + 1).unwrap(),
        Color::Green => write!(buf, "\x1b[{}m", base + 2).unwrap(),
        Color::Yellow => write!(buf, "\x1b[{}m", base + 3).unwrap(),
        Color::Blue => write!(buf, "\x1b[{}m", base + 4).unwrap(),
        Color::Magenta => write!(buf, "\x1b[{}m", base + 5).unwrap(),
        Color::Cyan => write!(buf, "\x1b[{}m", base + 6).unwrap(),
        Color::White => write!(buf, "\x1b[{}m", base + 7).unwrap(),
        Color::Gray => write!(buf, "\x1b[{}m", base + 60).unwrap(),
        Color::LightRed => write!(buf, "\x1b[{}m", base + 61).unwrap(),
        Color::LightGreen => write!(buf, "\x1b[{}m", base + 62).unwrap(),
        Color::LightYellow => write!(buf, "\x1b[{}m", base + 63).unwrap(),
        Color::LightBlue => write!(buf, "\x1b[{}m", base + 64).unwrap(),
        Color::LightMagenta => write!(buf, "\x1b[{}m", base + 65).unwrap(),
        Color::LightCyan => write!(buf, "\x1b[{}m", base + 66).unwrap(),
        Color::LightGray => write!(buf, "\x1b[{}m", base + 67).unwrap(),
        Color::Rgb(r, g, b) => {
            write!(buf, "\x1b[{};2;{};{};{}m", base + 8, r, g, b).unwrap();
        }
        Color::Indexed(idx) => {
            write!(buf, "\x1b[{};5;{}m", base + 8, idx).unwrap();
        }
    }
}
