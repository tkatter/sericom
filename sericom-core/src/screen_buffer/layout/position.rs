#![allow(unused)]

#[derive(Debug, Default, Clone, Copy, Eq, PartialEq, Hash)]
pub struct Position {
    pub(crate) x: u16,
    pub(crate) y: u16,
}

impl Position {
    pub const ORIGIN: Self = Self { x: 0, y: 0 };
}

impl From<(u16, u16)> for Position {
    fn from(value: (u16, u16)) -> Self {
        Self {
            x: value.0,
            y: value.1,
        }
    }
}
