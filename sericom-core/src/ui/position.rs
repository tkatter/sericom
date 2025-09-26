#![allow(unused)]

use std::fmt::Display;

use crate::screen_buffer::ScreenBuffer;

#[derive(Debug, Default, Clone, Copy, Eq, PartialEq, Hash)]
pub(crate) struct Position {
    pub(crate) x: u16,
    pub(crate) y: u16,
}

impl Position {
    pub const ORIGIN: Self = Self { x: 0, y: 0 };

    pub const fn set_y(&mut self, y: u16) {
        self.y = y;
    }
    pub const fn set_x(&mut self, x: u16) {
        self.x = x;
    }
    pub fn set_pos_from<P: Into<Position>>(&mut self, pos: P) {
        let p = pos.into();
        self.x = p.x;
        self.y = p.y;
    }
    pub const fn set_pos(&mut self, pos: Self) {
        *self = pos;
    }
    pub const fn get_x(&self) -> u16 {
        self.x
    }
    pub const fn get_y(&self) -> u16 {
        self.y
    }
}

impl From<(u16, u16)> for Position {
    fn from((x, y): (u16, u16)) -> Self {
        Self { x, y }
    }
}

impl From<(u16, usize)> for Position {
    fn from((x, y): (u16, usize)) -> Self {
        let y: u16 = y.try_into().expect("Out of scrollback buffer bounds");
        Self { x, y }
    }
}

impl From<Position> for (u16, usize) {
    fn from(position: Position) -> Self {
        (position.x, usize::from(position.y))
    }
}

impl From<Position> for (usize, u16) {
    fn from(position: Position) -> Self {
        (usize::from(position.x), position.y)
    }
}

impl From<Position> for (u16, u16) {
    fn from(position: Position) -> Self {
        (position.x, position.y)
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
