use crossterm::style::Color;

use crate::configs::get_config;

/// `Cell` represents a cell within the terminal's window/frame.
///
/// Used to hold rendering state for all the cells within the [`ScreenBuffer`][`super::ScreenBuffer`].
/// Each line within [`ScreenBuffer`][`super::ScreenBuffer`] is represented by a `Vec<Cell>`.
#[derive(Clone, Debug)]
pub struct Cell {
    pub(super) character: char,
    pub(super) fg_color: Color,
    pub(super) bg_color: Color,
    pub(super) is_selected: bool,
}

impl Default for Cell {
    /// The default for [`Cell`] is the fg color from [`Appearance.fg`][`crate::configs::Appearance`],
    /// the bg color from [`Appearance.bg`][`crate::configs::Appearance`], `' '` for the character, and is not selected.
    fn default() -> Self {
        let config = get_config();
        Self {
            character: ' ',
            fg_color: Color::from(&config.appearance.fg),
            bg_color: Color::from(&config.appearance.bg),
            is_selected: false,
        }
    }
}
