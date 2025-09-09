use crossterm::style::Color;

use crate::configs::get_config;

/// `Cell` represents a cell within the terminal's window/frame.
/// Used to hold rendering state for all the cells within the [`ScreenBuffer`].
/// Each line within `ScreenBuffer` is represented by a `Vec<Cell>`.
#[derive(Clone, Debug)]
pub(super) struct Cell {
    pub(super) character: char,
    pub(super) fg_color: Color,
    pub(super) bg_color: Color,
    pub(super) is_selected: bool,
}

impl Default for Cell {
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
