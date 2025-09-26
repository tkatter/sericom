#![allow(unused)]
use super::Line;
use super::Rect;
use crate::screen_buffer::Cell;

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Buffer {
    area: Rect,
    content: Vec<Line>,
}
impl Buffer {
    pub fn reset(&mut self) {
        for line in &mut self.content {
            line.reset();
        }
    }
    #[must_use]
    pub fn empty(area: Rect) -> Self {
        Self::filled(area, Cell::EMPTY)
    }

    #[must_use]
    pub fn filled(area: Rect, cell: Cell) -> Self {
        let line = Line::new(area.width.into(), cell);
        let size = area.height as usize;
        let content = vec![line; size];
        Self { area, content }
    }
}
