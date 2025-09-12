use crossterm::style::{Attribute, Attributes, Color, ContentStyle};

use crate::configs::get_config;

/// `Cell` represents a cell within the terminal's window/frame.
///
/// Used to hold rendering state for all the cells within the [`ScreenBuffer`][`super::ScreenBuffer`].
/// Each line within [`ScreenBuffer`][`super::ScreenBuffer`] is represented by a `Vec<Cell>`.
#[derive(Clone, Debug)]
pub struct Cell {
    pub(super) character: char,
    pub(super) style: ContentStyle,
    pub(super) is_selected: bool,
}

impl Cell {
    pub fn get_fg(&self) -> Color {
        self.style.foreground_color.unwrap()
    }
    pub fn get_bg(&self) -> Color {
        self.style.background_color.unwrap()
    }
    pub fn is_bold(&self) -> bool {
        self.style.attributes.has(Attribute::Bold)
    }
    pub fn set_bold(&mut self) {
        self.style.attributes.toggle(Attribute::Bold);
    }
    pub fn no_bold(&mut self) {
        self.style.attributes.unset(Attribute::NormalIntensity);
    }
    pub fn reverse(&mut self) {
        self.style.attributes.set(Attribute::Reverse);
    }
    pub fn unreverse(&mut self) {
        self.style.attributes.unset(Attribute::Reverse);
    }
}

impl Default for Cell {
    /// The default for [`Cell`] is the fg color from [`Appearance.fg`][`crate::configs::Appearance`],
    /// the bg color from [`Appearance.bg`][`crate::configs::Appearance`], `' '` for the character, and is not selected.
    fn default() -> Self {
        let config = get_config();
        
        let style = ContentStyle {
            foreground_color: Some(Color::from(&config.appearance.fg)),
            background_color: Some(Color::from(&config.appearance.bg)),
            attributes: Attributes::none(),
            underline_color: None,
        };
        Self {
            character: ' ',
            style,
            is_selected: false,
        }
    }
}
