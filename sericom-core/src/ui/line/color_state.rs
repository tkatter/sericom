use crossterm::{
    queue,
    style::{Color, Colored, Colors, Print, SetColors},
};
use miette::IntoDiagnostic;
use std::io::Write;

pub struct ColorState {
    colors: Colors,
}

impl Default for ColorState {
    fn default() -> Self {
        Self {
            colors: Colors::new(Color::Green, Color::Reset),
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

    pub fn set_colors(&mut self, ascii_str: &str) {
        self.colors = match Colored::parse_ansi(ascii_str) {
            Some(colored) => self.colors.then(&colored.into()),
            None => self.colors,
        };
    }
}
