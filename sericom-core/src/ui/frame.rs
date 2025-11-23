#![allow(unused)]

use crate::screen::TermPos;
use crate::{screen::ScreenBuffer, ui::Buffer};

use crate::{screen::Position, screen::Rect};

#[derive(Debug)]
pub struct Frame<'a> {
    pub(crate) buffer: &'a mut Buffer,
    pub(crate) cursor_position: Option<Position<TermPos>>,
    pub(crate) area: Rect<TermPos>,
}

impl Frame<'_> {
    #[must_use]
    pub const fn area(&self) -> Rect<TermPos> {
        self.area
    }

    pub fn render_widget<W: Widget>(&mut self, widget: W, area: Rect<TermPos>) {
        widget.render(area, self.buffer);
    }
}

pub trait Widget {
    fn render(self, area: Rect<TermPos>, buf: &mut Buffer)
    where
        Self: Sized;
}

impl Widget for ScreenBuffer {
    fn render(self, area: Rect<TermPos>, buf: &mut Buffer)
    where
        Self: Sized,
    {
    }
}
