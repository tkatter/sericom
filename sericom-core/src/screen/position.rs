use std::{fmt::Display, marker::PhantomData};

use super::Rect;
use super::ScreenBuffer;

const TAB_WIDTH: u16 = 8;

#[derive(Debug, Default, Clone, Copy, Eq, PartialEq, Hash)]
pub struct TermPos;
#[derive(Debug, Default, Clone, Copy, Eq, PartialEq, Hash)]
pub struct BuffPos;

pub trait Scope {}
impl Scope for TermPos {}
impl Scope for BuffPos {}

pub trait PosType {
    type Y: Copy
        + std::hash::Hash
        + From<u16>
        + std::fmt::Display
        + Clone
        + Default
        + Eq
        + PartialEq
        + std::fmt::Debug
        + From<u8>;
}

impl PosType for TermPos {
    type Y = u16;
}
impl PosType for BuffPos {
    type Y = u32;
}

pub type PosY<S> = <S as PosType>::Y;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct Position<S: Scope + PosType> {
    pub(crate) x: u16,
    pub(crate) y: PosY<S>,
    _phantom: PhantomData<S>,
}

impl Position<TermPos> {
    pub const ORIGIN: Self = Self {
        x: 0,
        y: 0,
        _phantom: PhantomData,
    };
}

impl<S: Scope + PosType> Position<S> {
    pub const fn tabn(&mut self, n: u16) {
        let mut acc = 0;
        while acc < n {
            self.tab();
            acc += 1;
        }
    }
    pub const fn rtabn(&mut self, n: u16) {
        let mut acc = 0;
        while acc < n {
            self.rtab();
            acc += 1;
        }
    }
    pub const fn tab(&mut self) {
        let tab = TAB_WIDTH - (self.x % TAB_WIDTH);
        self.x += tab;
    }
    pub const fn rtab(&mut self) {
        let tab = TAB_WIDTH - (self.x % TAB_WIDTH);
        self.x.saturating_sub(tab);
    }
    pub const fn set_y(&mut self, y: PosY<S>) {
        self.y = y;
    }
    pub const fn set_x(&mut self, x: u16) {
        self.x = x;
    }
    pub fn set_pos_from<P: Into<Self>>(&mut self, pos: P) {
        let p = pos.into();
        self.x = p.x;
        self.y = p.y;
    }
    pub const fn set(&mut self, pos: Self) {
        *self = pos;
    }
    pub const fn x(&self) -> u16 {
        self.x
    }
    pub const fn y(&self) -> PosY<S> {
        self.y
    }
}

impl<S> Default for Position<S>
where
    S: Scope + PosType,
    <S as PosType>::Y: Default,
{
    fn default() -> Self {
        Self {
            x: 0,
            y: Default::default(),
            _phantom: PhantomData,
        }
    }
}

impl<S: Scope + PosType> Display for Position<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}, {})", self.x, self.y)
    }
}

macro_rules! impl_from {
    ($from:ty, $for:path) => {
        impl From<$from> for $for {
            fn from((x, y): $from) -> Self {
                Self {
                    x,
                    y,
                    _phantom: PhantomData,
                }
            }
        }
    };
    ($from:ty, $for:path, y_as = $as:ty) => {
        impl From<$from> for $for {
            fn from((x, y): $from) -> Self {
                Self {
                    x,
                    y: <$as>::try_from(y).expect("usize is within the height of terminal"),
                    _phantom: PhantomData,
                }
            }
        }
    };
    ($from:ty, $for:path, x_as = $as:ty) => {
        impl From<$from> for $for {
            fn from((x, y): $from) -> Self {
                Self {
                    x: <$as>::try_from(x).expect("usize is within the width of terminal"),
                    y,
                    _phantom: PhantomData,
                }
            }
        }
    };
}

//-- TermPos --//
impl_from!((u16, u16), Position<TermPos>);
impl_from!((u16, usize), Position<TermPos>, y_as = u16);
impl_from!((usize, u16), Position<TermPos>, x_as = u16);

// -- BuffPos --//
impl_from!((u16, u32), Position<BuffPos>);
impl_from!((u16, usize), Position<BuffPos>, y_as = u32);

pub trait HasBounds {
    type Bounds;
    fn bounds(&self) -> Self::Bounds;
}

impl HasBounds for ScreenBuffer {
    type Bounds = Rect<TermPos>;

    fn bounds(&self) -> Self::Bounds {
        self.rect
    }
}

/// Coordinates movement within the implementor based on the implementor's bounds.
///
/// Decide what the implementor's bounds are, and implement [`HasBounds`]. Then
/// this trait can be implemented once and provide uniform movement logic throughout
/// your code based on how the movement should behave within your type's bounds.
pub trait Cursor: HasBounds {
    /// Set both the x and y of the cursor's position.
    fn set_cursor_pos<P>(&mut self, position: P)
    where
        P: Into<Position<TermPos>>;
    /// Move the cursor left relative to it's current y position.
    fn move_cursor_left(&mut self, cells: u16);
    /// Move the cursor right relative to it's current x position.
    fn move_cursor_right(&mut self, cells: u16);
    /// Move the cursor up relative to it's current y position.
    fn move_cursor_up(&mut self, lines: u16);
    /// Move the cursor down relative to it's current y position.
    fn move_cursor_down(&mut self, lines: u16);
    /// Set the cursor's x position directly.
    ///
    /// Useful when the movement is not relative to the cursor's current
    /// position, but a direct/absolute position.
    fn set_cursor_col(&mut self, col: u16);
    /// Set the cursor's y position directly.
    ///
    /// Useful when the movement is not relative to the cursor's current
    /// position, but a direct/absolute position.
    fn set_cursor_row(&mut self, row: u16);
    fn tab(&mut self, n: u16);
    fn rtab(&mut self, n: u16);
}

impl Cursor for ScreenBuffer {
    fn tab(&mut self, n: u16) {
        let bounds = self.bounds();
        self.cursor.tabn(n);
        if self.cursor.x > bounds.right() {
            self.cursor.x = bounds.right();
        }
    }

    fn rtab(&mut self, n: u16) {
        self.cursor.tabn(n);
    }

    fn set_cursor_pos<P>(&mut self, position: P)
    where
        P: Into<Position<TermPos>>,
    {
        let bounds = self.bounds();
        let mut new_pos: Position<TermPos> = position.into();
        if new_pos.x > bounds.right() {
            new_pos.x = bounds.right();
        }
        if new_pos.y > bounds.bottom() {
            new_pos.y = bounds.bottom();
        }

        self.cursor.set_pos_from(new_pos);
    }

    fn move_cursor_left(&mut self, cells: u16) {
        self.cursor.x = self.cursor.x.saturating_sub(cells);
    }

    fn move_cursor_right(&mut self, cells: u16) {
        let bounds = self.bounds();
        let mut new_x = self.cursor.x.saturating_add(cells);
        if new_x > bounds.right() {
            new_x = bounds.right();
        }
        self.cursor.x = new_x;
    }

    fn move_cursor_up(&mut self, lines: u16) {
        self.cursor.y = self.cursor.y.saturating_sub(lines);
    }

    fn move_cursor_down(&mut self, lines: u16) {
        let bounds = self.bounds();
        let new_y = self.cursor.y.saturating_add(lines);

        // TODO: FIGURE OUT LINE PUSHING
        if new_y <= bounds.bottom() {
            self.cursor.y = new_y;
        }
    }

    fn set_cursor_col(&mut self, col: u16) {
        let bounds = self.bounds();
        if col > bounds.right() {
            self.cursor.x = bounds.right();
        } else {
            self.cursor.x = col;
        }
    }

    #[allow(unused)]
    fn set_cursor_row(&mut self, row: u16) {
        unimplemented!()
    }
}

pub trait TranslatePos {
    fn to_term(&self, pos: Position<BuffPos>) -> Position<TermPos>;
    fn to_buff(&self, pos: Position<TermPos>) -> Position<BuffPos>;
}

impl TranslatePos for ScreenBuffer {
    fn to_term(&self, pos: Position<BuffPos>) -> Position<TermPos> {
        let buff_win = self.buff_rect();

        let visible_y = pos.y.clamp(buff_win.top(), buff_win.bottom());

        // Casting is fine because the viewport (buff_win) is the size of
        // a user's terminal and visible_y - buff_win.top() simply returns
        // a number somewhere within the height of the terminal
        #[allow(clippy::cast_possible_truncation)]
        let term_y = (visible_y - buff_win.top()) as u16;

        let term_x = pos.x.clamp(buff_win.left(), buff_win.right());

        Position::<TermPos>::from((term_x, term_y))
    }

    fn to_buff(&self, pos: Position<TermPos>) -> Position<BuffPos> {
        let buff_y: u32 = self.view_start + u32::from(pos.y);
        let buff_x = pos.x.clamp(0, self.width());

        Position::<BuffPos>::from((buff_x, buff_y))
    }
}
