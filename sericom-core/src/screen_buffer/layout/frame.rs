#![allow(unused)]

use super::{super::Buffer, position::Position, rect::Rect};

#[derive(Debug)]
pub struct Frame<'a> {
    pub(crate) buffer: &'a mut Buffer,
    pub(crate) cursor_position: Option<Position>,
    pub(crate) area: Rect,
}

impl Frame<'_> {
    #[must_use]
    pub const fn area(&self) -> Rect {
        self.area
    }

    pub fn render_widget<W: Widget>(&mut self, widget: W, area: Rect) {
        widget.render(area, self.buffer);
    }
}

pub trait Widget {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized;
}
