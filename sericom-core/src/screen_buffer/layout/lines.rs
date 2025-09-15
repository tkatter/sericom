#![allow(unused)]

use super::rect::Rect;

pub struct Lines {
    rect: Rect,
    curr_line_fwd: u16,
    curr_line_back: u16,
}

impl Lines {
    #[must_use]
    pub const fn new(rect: Rect) -> Self {
        Self {
            rect,
            curr_line_fwd: rect.top(),
            curr_line_back: rect.bottom(),
        }
    }
}

impl Iterator for Lines {
    type Item = Rect;
    fn next(&mut self) -> Option<Self::Item> {
        if self.curr_line_fwd >= self.curr_line_back {
            return None;
        }
        let row = Rect::new(
            (self.rect.left(), self.curr_line_fwd).into(),
            self.rect.width,
            1,
        );
        self.curr_line_fwd += 1;
        Some(row)
    }
}

impl DoubleEndedIterator for Lines {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.curr_line_back <= self.curr_line_fwd {
            return None;
        }
        self.curr_line_back -= 1;
        let row = Rect::new(
            (self.rect.left(), self.curr_line_back).into(),
            self.rect.width,
            1,
        );
        Some(row)
    }
}
