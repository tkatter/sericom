use std::ops::{Deref, DerefMut};

/// `Cell` represents a cell within the terminal's window/frame.
///
/// Used to hold rendering state for all the cells within the [`ScreenBuffer`][`super::ScreenBuffer`].
/// Each line within [`ScreenBuffer`][`super::ScreenBuffer`] is represented by a `Vec<Cell>`.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Cell {
    /// Stored as u8 for memory size optimization (1 byte vs 4)
    ///
    /// [`Cell`] is only constructed from valid ascii characters from the [`ByteParser`]
    /// and since it is only ascii characters, it is safe to store as [`u8`] and
    /// later cast as char. This reduces the size of [`Cell`] from 8 to 2 (if [`char`]
    /// were used instead of [`u8`]).
    pub(crate) character: u8,
    pub(crate) is_selected: bool,
}

impl Cell {
    pub const CARRIGE: Self = Self::new(b'\r');
    pub const EMPTY: Self = Self::new(b' ');
    pub const NEWLINE: Self = Self::new(b'\n');
    pub const TAB: Self = Self::new(b'\t');

    #[must_use]
    pub const fn new(character: u8) -> Self {
        Self {
            character,
            is_selected: false,
        }
    }
}

impl Deref for Cell {
    type Target = u8;

    fn deref(&self) -> &Self::Target {
        &self.character
    }
}

impl DerefMut for Cell {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.character
    }
}

impl From<char> for Cell {
    /// WARNING: This will have unexpected behavior if the `char` is _NOT_ valid ASCII
    fn from(value: char) -> Self {
        Self::new(value as u8)
    }
}

impl From<&char> for Cell {
    /// WARNING: This will have unexpected behavior if the `char` is _NOT_ valid ASCII
    fn from(value: &char) -> Self {
        Self::new(*value as u8)
    }
}

impl From<u8> for Cell {
    fn from(value: u8) -> Self {
        Self::new(value)
    }
}

impl From<&u8> for Cell {
    fn from(value: &u8) -> Self {
        Self::new(*value)
    }
}

impl From<Cell> for char {
    fn from(value: Cell) -> Self {
        value.character as Self
    }
}

impl From<Cell> for u8 {
    fn from(value: Cell) -> Self {
        value.character
    }
}

impl Default for Cell {
    /// Default is a [space] char (' ') and is_selected = false
    fn default() -> Self {
        Self {
            character: b' ',
            is_selected: false,
        }
    }
}

impl<'a> FromIterator<&'a Cell> for std::string::String {
    fn from_iter<T: IntoIterator<Item = &'a Cell>>(iter: T) -> Self {
        iter.into_iter()
            .map(|c| c.character as char)
            .collect::<Self>()
    }
}
