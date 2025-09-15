#![allow(unused)]
use std::cmp::{max, min};

use super::position::Position;

// #[derive(Debug, Clone, Eq, PartialEq)]
#[derive(Debug, Default, Clone, Copy, Eq, PartialEq, Hash)]
pub struct Rect {
    pub(crate) width: u16,
    pub(crate) height: u16,
    pub(crate) origin: Position,
}

impl Rect {
    #[must_use]
    pub const fn new(origin: Position, width: u16, height: u16) -> Self {
        Self {
            width,
            height,
            origin,
        }
    }

    #[must_use]
    pub const fn left(&self) -> u16 {
        self.origin.x
    }

    #[must_use]
    pub const fn right(&self) -> u16 {
        self.origin.x.saturating_add(self.width)
    }

    #[must_use]
    pub const fn top(&self) -> u16 {
        self.origin.y
    }

    #[must_use]
    pub const fn bottom(&self) -> u16 {
        self.origin.y.saturating_add(self.height)
    }

    #[must_use]
    pub const fn area(&self) -> u16 {
        self.width.saturating_mul(self.height)
    }

    #[must_use]
    pub fn intersection(&self, other: Self) -> Self {
        let x1 = max(self.left(), other.left());
        let x2 = min(self.right(), other.right());
        let y1 = max(self.top(), other.top());
        let y2 = min(self.bottom(), other.bottom());
        Self {
            origin: (x1, y1).into(),
            width: x2.saturating_sub(x1),
            height: y2.saturating_sub(y1),
        }
    }
}

impl From<(u16, u16)> for Rect {
    /// Creates a new `Rect` with (width, height) at [`Position::ORIGIN`]
    fn from(value: (u16, u16)) -> Self {
        Self {
            width: value.0,
            height: value.1,
            origin: Position::ORIGIN,
        }
    }
}
