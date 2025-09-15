#![allow(unused)]
use super::{Cell, Line, layout::rect::Rect};

#[derive(Debug, Clone)]
pub struct Buffer {
    area: Rect,
    content: Vec<Line>,
}
impl Buffer {
    #[must_use]
    pub fn empty(area: Rect) -> Self {
        Self::filled(area, Cell::EMPTY)
    }

    #[must_use]
    pub fn filled(area: Rect, cell: Cell) -> Self {
        let line = Line::new(area.width as usize, cell);
        let size = area.height as usize;
        let content = vec![line; size];
        Self { area, content }
    }
}
