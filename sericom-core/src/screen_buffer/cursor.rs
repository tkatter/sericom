use std::fmt::Display;

use super::{Line, ScreenBuffer};

/// Represent's the cursor's position within the [`ScreenBuffer`].
#[derive(Clone, Copy, Debug)]
pub struct Position {
    /// The column within [`ScreenBuffer`]'s scrollback buffer.
    /// This translates to the [`Cell`][`super::Cell`] within a line (`Vec`).
    pub(crate) x: u16,
    /// The line number within [`ScreenBuffer`]'s scrollback buffer.
    pub(crate) y: usize,
}

impl Position {
    /// Sets [`Position`] to (0,0)
    pub(super) const fn home() -> Self {
        Self { x: 0, y: 0 }
    }
}

impl From<(u16, usize)> for Position {
    fn from((x, y): (u16, usize)) -> Self {
        Self { x, y }
    }
}

impl From<(u16, u16)> for Position {
    fn from((x, y): (u16, u16)) -> Self {
        Self { x, y: y as usize }
    }
}

impl From<Position> for (u16, usize) {
    fn from(position: Position) -> Self {
        (position.x, position.y)
    }
}

#[allow(clippy::cast_possible_truncation)]
impl From<Position> for (u16, u16) {
    fn from(position: Position) -> Self {
        (position.x, position.y as u16)
    }
}

impl Display for Position {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}, {})", self.x, self.y)
    }
}

pub trait Cursor {
    fn set_cursor_pos<P: Into<Position>>(&mut self, position: P);
    fn move_cursor_left(&mut self, cells: u16);
    fn move_cursor_up(&mut self, lines: u16);
    fn move_cursor_down(&mut self, lines: u16);
    fn move_cursor_right(&mut self, cells: u16);
    fn set_cursor_col(&mut self, col: u16);
}

impl Cursor for ScreenBuffer {
    /// Sets the cursor position.
    fn set_cursor_pos<P: Into<Position>>(&mut self, position: P) {
        self.cursor_pos = position.into();
    }

    /// Moves the cursor left by `cells`.
    fn move_cursor_left(&mut self, cells: u16) {
        self.cursor_pos.x = self.cursor_pos.x.saturating_sub(cells);
    }

    /// Moves the cursor up by `lines`.
    fn move_cursor_up(&mut self, lines: u16) {
        self.cursor_pos.y = self.cursor_pos.y.saturating_sub(lines as usize);
    }

    /// Moves the cursor down by `lines`.
    fn move_cursor_down(&mut self, lines: u16) {
        self.cursor_pos.y = self.cursor_pos.y.saturating_add(lines as usize);
        while self.cursor_pos.y > self.lines.len() {
            self.lines.push_back(Line::new_default(self.width as usize));
        }
    }

    /// Moves the cursor right by `cells`.
    fn move_cursor_right(&mut self, cells: u16) {
        self.cursor_pos.x = self.cursor_pos.x.saturating_add(cells);
    }

    /// Sets the column of the cursor
    fn set_cursor_col(&mut self, col: u16) {
        self.cursor_pos.x = col;
    }
}
