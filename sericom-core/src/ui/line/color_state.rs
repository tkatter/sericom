use crossterm::{
    queue,
    style::{Color, Colored, Colors, Print, SetColors},
};
use miette::IntoDiagnostic;
use std::io::Write;

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
    pub fn get_colors(&self) -> Colors {
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
        self.colors = match Colored::parse_ansi(ascii_str) {
            Some(colored) => {
                eprintln!("{:#?}", colored);
                self.colors.then(&colored.into())
            }
            None => {
                match Color::parse_ansi(ascii_str) {
                    Some(color) => eprintln!("Second try got: {:#?}", color),
                    None => eprintln!("Failed to parse ascii_str second time"),
                }
                eprintln!("Failed to parse ascii_str");
                self.colors
            }
        };
    }
}
