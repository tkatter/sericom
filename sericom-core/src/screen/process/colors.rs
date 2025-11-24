use crossterm::{
    queue,
    style::{Attribute, Attributes, Color, Colored, Colors, Print, SetColors},
};
use miette::IntoDiagnostic;
use std::io::Write;

use super::SEP;
use crate::configs::get_config;

#[derive(Debug)]
pub struct ColorState {
    colors: Colors,
}

impl Default for ColorState {
    fn default() -> Self {
        let config = get_config();
        let fg = Color::from(&config.appearance.fg);
        let bg = Color::from(&config.appearance.bg);
        Self {
            colors: Colors::new(fg, bg),
        }
    }
}

impl ColorState {
    #[must_use]
    pub const fn get_colors(&self) -> Colors {
        self.colors
    }
    pub fn queue_line<W: Write>(&self, writer: &mut W, text: &str) -> miette::Result<()> {
        queue!(writer, SetColors(self.colors), Print(text)).into_diagnostic()?;
        Ok(())
    }
    pub fn set_fg(&mut self, color: Color) {
        self.colors = Colors::new(color, self.colors.background.unwrap_or(Color::Reset));
    }
    pub fn set_bg(&mut self, color: Color) {
        self.colors = Colors::new(self.colors.foreground.unwrap_or(Color::Reset), color);
    }
    pub fn reset(&mut self) {
        self.colors = Colors::new(Color::Reset, Color::Reset);
    }
    pub fn set_colors(&mut self, ascii_str: &str) {
        self.colors = if let Some(colored) = Colored::parse_ansi(ascii_str) {
            eprintln!("{colored:#?}");
            self.colors.then(&colored.into())
        } else {
            match Color::parse_ansi(ascii_str) {
                Some(color) => eprintln!("Second try got: {color:#?}"),
                None => eprintln!("Failed to parse ascii_str second time"),
            }
            eprintln!("Failed to parse ascii_str");
            self.colors
        };
    }
}

pub fn process_colors(seq: &[u8], color_state: &mut ColorState, attrs: &mut Attributes) {
    // Get the part between 'ESC[' and 'm'
    let body = &seq[2..seq.len() - 1];
    let mut body_idx = 0;
    let mut parts_iter = body.split(|&p| p == SEP).peekable();

    while let Some(part) = parts_iter.next() {
        match part {
            // Give any 38;5;26 / 48;2;50;60;70 sequences to crossterm
            [b'3' | b'4', b'8'] => {
                if let Some(ident) = parts_iter.next_if(|n| *n == [b'5']) {
                    let Some(color) = parts_iter.next() else {
                        break;
                    };

                    // Get slice from body (+ 1 for the separators ';')
                    let part_str_len = part.len() + 1 + ident.len() + 1 + color.len();
                    let slice = &body[body_idx..body_idx + part_str_len];

                    #[cfg(test)]
                    eprintln!("slice: {}", str::from_utf8(slice).unwrap());

                    if let Ok(s) = std::str::from_utf8(slice) {
                        color_state.set_colors(s);
                    }

                    // add to body_idx the indexes we consumed
                    body_idx += part_str_len - part.len();
                } else if let Some(ident) = parts_iter.next_if(|n| *n == [b'2']) {
                    let Some(r) = parts_iter.next() else { break };
                    let Some(g) = parts_iter.next() else { break };
                    let Some(b) = parts_iter.next() else { break };

                    // get slice from body (+ 1 for the separators ';')
                    let part_str_len =
                        part.len() + 1 + ident.len() + 1 + r.len() + 1 + g.len() + 1 + b.len();
                    let slice = &body[body_idx..body_idx + part_str_len];

                    #[cfg(test)]
                    eprintln!("slice: {}", str::from_utf8(slice).unwrap());

                    if let Ok(s) = std::str::from_utf8(slice) {
                        color_state.set_colors(s);
                    }

                    // add to body_idx the indexes we consumed
                    body_idx += part_str_len - part.len();
                }
            }
            _ => handle_colors_and_attrs(part, color_state, attrs),
        }
        // + 1 for the separator
        body_idx += part.len() + 1;
    }
}

fn handle_colors_and_attrs(body: &[u8], color_state: &mut ColorState, attrs: &mut Attributes) {
    match *body {
        // Attribute sgr mapping to crossterm
        [b'0'] => {
            *color_state = ColorState::default(); // uses config colors
            *attrs = Attributes::default();
        }
        [b'1'] => attrs.set(Attribute::Bold),
        [b'2'] => attrs.set(Attribute::Dim),
        [b'3'] => attrs.set(Attribute::Italic),
        [b'4'] => attrs.set(Attribute::Underlined),
        [b'4', b':', b'2'] => attrs.set(Attribute::DoubleUnderlined),
        [b'4', b':', b'3'] => attrs.set(Attribute::Undercurled),
        [b'4', b':', b'4'] => attrs.set(Attribute::Underdotted),
        [b'4', b':', b'5'] => attrs.set(Attribute::Underdashed),
        [b'5'] => attrs.set(Attribute::SlowBlink),
        [b'6'] => attrs.set(Attribute::RapidBlink),
        [b'7'] => attrs.set(Attribute::Reverse),
        [b'8'] => attrs.set(Attribute::Hidden),
        [b'9'] => attrs.set(Attribute::CrossedOut),
        [b'2', b'0'] => attrs.set(Attribute::Fraktur),
        [b'2', b'1'] => attrs.set(Attribute::NoBold),
        [b'2', b'2'] => attrs.set(Attribute::NormalIntensity),
        [b'2', b'3'] => attrs.set(Attribute::NoItalic),
        [b'2', b'4'] => attrs.set(Attribute::NoUnderline),
        [b'2', b'5'] => attrs.set(Attribute::NoBlink),
        [b'2', b'7'] => attrs.set(Attribute::NoReverse),
        [b'2', b'8'] => attrs.set(Attribute::NoHidden),
        [b'2', b'9'] => attrs.set(Attribute::NotCrossedOut),
        [b'5', b'1'] => attrs.set(Attribute::Framed),
        [b'5', b'2'] => attrs.set(Attribute::Encircled),
        [b'5', b'3'] => attrs.set(Attribute::OverLined),
        [b'5', b'4'] => attrs.set(Attribute::NotFramedOrEncircled),
        [b'5', b'5'] => attrs.set(Attribute::NotOverLined),
        // Basic 8 foreground
        [b'3', b'0'] => color_state.set_fg(Color::Black),
        [b'3', b'1'] => color_state.set_fg(Color::DarkRed),
        [b'3', b'2'] => color_state.set_fg(Color::DarkGreen),
        [b'3', b'3'] => color_state.set_fg(Color::DarkYellow),
        [b'3', b'4'] => color_state.set_fg(Color::DarkBlue),
        [b'3', b'5'] => color_state.set_fg(Color::DarkMagenta),
        [b'3', b'6'] => color_state.set_fg(Color::DarkCyan),
        [b'3', b'7'] => color_state.set_fg(Color::Grey),
        // Basic 8 background
        [b'4', b'0'] => color_state.set_bg(Color::Black),
        [b'4', b'1'] => color_state.set_bg(Color::DarkRed),
        [b'4', b'2'] => color_state.set_bg(Color::DarkGreen),
        [b'4', b'3'] => color_state.set_bg(Color::DarkYellow),
        [b'4', b'4'] => color_state.set_bg(Color::DarkBlue),
        [b'4', b'5'] => color_state.set_bg(Color::DarkMagenta),
        [b'4', b'6'] => color_state.set_bg(Color::DarkCyan),
        [b'4', b'7'] => color_state.set_bg(Color::Grey),
        // Basic 8 bright foreground
        [b'9', b'0'] => color_state.set_fg(Color::DarkGrey),
        [b'9', b'1'] => color_state.set_fg(Color::Red),
        [b'9', b'2'] => color_state.set_fg(Color::Green),
        [b'9', b'3'] => color_state.set_fg(Color::Yellow),
        [b'9', b'4'] => color_state.set_fg(Color::Blue),
        [b'9', b'5'] => color_state.set_fg(Color::Magenta),
        [b'9', b'6'] => color_state.set_fg(Color::Cyan),
        [b'9', b'7'] => color_state.set_fg(Color::White),
        // Basic 8 bright background
        [b'1', b'0', b'0'] => color_state.set_bg(Color::DarkGrey),
        [b'1', b'0', b'1'] => color_state.set_bg(Color::Red),
        [b'1', b'0', b'2'] => color_state.set_bg(Color::Green),
        [b'1', b'0', b'3'] => color_state.set_bg(Color::Yellow),
        [b'1', b'0', b'4'] => color_state.set_bg(Color::Blue),
        [b'1', b'0', b'5'] => color_state.set_bg(Color::Magenta),
        [b'1', b'0', b'6'] => color_state.set_bg(Color::Cyan),
        [b'1', b'0', b'7'] => color_state.set_bg(Color::White),
        // Set colors to default
        [b'3', b'9'] => color_state.set_fg(Color::Reset),
        [b'4', b'9'] => color_state.set_bg(Color::Reset),
        _ => {} // Ignore rest
    }
}
