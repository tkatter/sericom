use crossterm::style::Color;

use crate::configs::get_config;

/// `Cell` represents a cell within the terminal's window/frame.
///
/// Used to hold rendering state for all the cells within the [`ScreenBuffer`][`super::ScreenBuffer`].
/// Each line within [`ScreenBuffer`][`super::ScreenBuffer`] is represented by a `Vec<Cell>`.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Cell {
    pub(crate) character: char,
    pub(crate) is_selected: bool,
}

impl Cell {
    pub const EMPTY: Self = Self::new(' ');

    #[must_use]
    pub const fn new(character: char) -> Self {
        Self {
            character,
            is_selected: false,
        }
    }
}

impl From<char> for Cell {
    fn from(value: char) -> Self {
        Self::new(value)
    }
}

impl From<Cell> for char {
    fn from(value: Cell) -> Self {
        value.character
    }
}

impl Default for Cell {
    /// The default for [`Cell`] is the fg color from [`Appearance.fg`][`crate::configs::Appearance`],
    /// the bg color from [`Appearance.bg`][`crate::configs::Appearance`], `' '` for the character, and is not selected.
    fn default() -> Self {
        Self {
            character: ' ',
            is_selected: false,
        }
    }
}
