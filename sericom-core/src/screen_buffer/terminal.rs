/*
 * Example workflow:
 *
 * Initialize terminal:
 *
 * Terminal::new(
 *     [Buffer::new(), Buffer::new()], // Initialize Buffers
 *     terminal::size() // Size of terminal from crossterm
 * );
 *
 * Data lifecycle:
 *
 * parser -> add_data() -> ScrollbackBuffer::lines.push(Line(Vec<Cell>)) -|
 *                                                                        |
 * |----------------------------------------------------------------------|
 * |
 * |-> Terminal::draw(|frame| { ScrollbackBuffer::render(frame.area, frame.buffer) }); -|
 *                                                                                      |
 * |------------------------------------------------------------------------------------|
 * |
 * |-> writes lines from ScrollbackBuffer::lines[Range<frame.area.height>] into frame.buffer -|
 *                                                                                            |
 * |------------------------------------------------------------------------------------------|
 * |
 * |-> for line in Terminal::buffers[current].iter() { /* draw line to stdout */ }
 *
 */
#![allow(unused)]
use super::{
    Buffer,
    layout::{frame::Frame, position::Position, rect::Rect},
};
use crate::screen_buffer::Cell;

// use std::collections::VecDeque;

#[derive(Debug, Clone)]
pub struct Terminal {
    buffers: [Buffer; 2],
    current_buffer: usize,
    cursor_hidden: bool,
    view_area: Rect,
}

impl Terminal {
    pub const fn get_frame(&mut self) -> Frame<'_> {
        Frame {
            buffer: &mut self.buffers[self.current_buffer],
            cursor_position: None,
            area: self.view_area,
        }
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

// #[derive(Debug, Clone)]
// pub struct ScrollbackBuffer {
//     lines: VecDeque<Line>,
//     max_scrollback: usize,
// }
