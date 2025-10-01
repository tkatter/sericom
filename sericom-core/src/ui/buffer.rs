#![allow(unused)]
use super::Line;
use super::Rect;
use crate::ui::Cell;
use crate::ui::Span;

#[derive(Debug, Clone, Eq, PartialEq)]
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
        Self::filled(area, Span::default())
    }

    #[must_use]
    pub fn filled(area: Rect, span: Span) -> Self {
        let line = Line::new(area.width.into(), span);
        let size = area.height as usize;
        let content = vec![line; size];
        Self { area, content }
    }
}
