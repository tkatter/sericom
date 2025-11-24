use std::cmp::{max, min};

use super::position::{BuffPos, PosType, PosY, Position, Scope, TermPos};

// #[derive(Debug, Clone, Eq, PartialEq)]
#[derive(Debug, Default, Clone, Copy, Eq, PartialEq, Hash)]
pub struct Rect<S: Scope + PosType> {
    pub(crate) width: u16,
    pub(crate) height: PosY<S>,
    pub(crate) origin: Position<S>,
}

impl<S: Scope + PosType> Rect<S> {
    #[must_use]
    pub const fn new(origin: Position<S>, width: u16, height: PosY<S>) -> Self {
        Self {
            width,
            height,
            origin,
        }
    }

    /// The x value of [`Rect`]s left side
    #[must_use]
    pub const fn left(&self) -> u16 {
        self.origin.x
    }

    /// The x value of [`Rect`]s right side
    #[must_use]
    pub const fn right(&self) -> u16 {
        self.origin.x.saturating_add(self.width)
    }

    /// The y value of [`Rect`]s top side
    #[must_use]
    pub const fn top(&self) -> PosY<S> {
        self.origin.y
    }
}

impl Rect<TermPos> {
    /// The y value of [`Rect`]s bottom side
    #[must_use]
    pub const fn bottom(&self) -> u16 {
        self.origin.y.saturating_add(self.height)
    }
}

// These methods are only relavant to [`Rect<TermPos>`]
impl Rect<TermPos> {
    /// The area (W x H) of [`Rect`]
    #[must_use]
    pub const fn area(&self) -> u32 {
        self.width as u32 * self.height as u32
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

impl Rect<BuffPos> {
    /// The y value of [`Rect`]s bottom side
    #[must_use]
    pub const fn bottom(&self) -> u32 {
        self.origin.y.saturating_add(self.height)
    }
}

impl<P, S> From<(P, u16, PosY<S>)> for Rect<S>
where
    P: Into<Position<S>>,
    S: Scope + PosType,
{
    /// Create a [`Rect`] from (origin, width, height)
    fn from(value: (P, u16, PosY<S>)) -> Self {
        Self {
            width: value.1,
            height: value.2,
            origin: value.0.into(),
        }
    }
}

impl From<(u16, u16)> for Rect<TermPos> {
    /// Creates a new `Rect` with (width, height) at [`Position::ORIGIN`]
    fn from(value: (u16, u16)) -> Self {
        Self {
            width: value.0,
            height: value.1,
            origin: Position::ORIGIN,
        }
    }
}
