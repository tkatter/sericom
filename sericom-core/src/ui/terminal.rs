use crate::ui::{Buffer, Frame};

use super::Rect;

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Terminal {
    buffers: [Buffer; 2],
    current_buffer: usize,
    cursor_hidden: bool,
    view_area: Rect,
}

impl Terminal {
    pub const fn get_frame(&mut self) -> Frame<'_> {
        Frame {
            buffer: &mut self.buffers[self.next_buf()],
            cursor_position: None,
            area: self.view_area,
        }
    }

    pub fn draw<F>(&mut self, f: F)
    where
        F: FnOnce(&mut Frame),
    {
        // Clear next buffer before drawing into
        self.buffers[self.next_buf()].reset();

        let mut frame = self.get_frame();
        f(&mut frame);
    }

    const fn next_buf(&self) -> usize {
        1 - self.current_buffer
    }

    pub(crate) const fn area(&self) -> Rect {
        self.view_area
    }
}

impl Default for Terminal {
    fn default() -> Self {
        use crossterm::terminal::size;

        let (term_w, term_y) = size().unwrap_or((80, 24));
        let view_area = Rect::from((term_w, term_y));
        let buffers = [Buffer::empty(view_area), Buffer::empty(view_area)];

        Self {
            buffers,
            current_buffer: 0_usize,
            cursor_hidden: false,
            view_area,
        }
    }
}
