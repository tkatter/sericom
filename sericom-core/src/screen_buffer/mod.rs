#![allow(clippy::too_long_first_doc_paragraph)]
//! This module contains the code needed for the implementation of a
//! stateful buffer that holds a history of the lines/data received
//! from the serial connection and the rendering/updating of the buffer
//! to the terminal screen (stdout).
//!
//! Simply writing the data received from the serial connection directly
//! to stdout creates one main issue: there is no history of previous lines
//! that were received from the serial connection. Without a screen buffer,
//! lines would simply be wiped from existence as they exit the terminal's screen.
//!
//! As a result, there would be no way to implement features like scrolling,
//! highlighting text (for UI purposes), and getting characters at specific
//! locations within the screen for things like copying to a clipboard.
//!
//! The screen buffer solves these issues by storing each line received from the
//! connection in a [`VecDeque`]. It is important to note that
//! currently, the **capacity of the [`VecDeque`] is hardcoded with a value of 10,000
//! lines with [`MAX_SCROLLBACK`]**.

mod buffer;
mod cell;
mod cursor;
mod escape;
pub mod layout;
mod line;
mod render;
pub mod terminal;
mod ui_command;
pub use buffer::*;
pub use cell::*;
use crossterm::style::Attributes;
pub use cursor::*;
use escape::{EscapeSequence, EscapeState};
pub use line::*;
pub use ui_command::*;

use std::collections::VecDeque;

/// The maximum number of lines stored in memory in [`ScreenBuffer`].
pub const MAX_SCROLLBACK: usize = 10000;

/// The `ScreenBuffer` holds rendering state for the entire terminal's window/frame.
///
/// It mainly serves to allow for user-interactions that require a history and location
/// of the data displayed within the terminal i.e. copy/paste, scrolling, & highlighting.
#[derive(Debug)]
pub struct ScreenBuffer {
    /// Terminal width
    width: u16,
    /// Terminal height
    height: u16,
    /// Scrollback buffer (all lines received from the serial connection).
    /// Limited by memory.
    lines: VecDeque<Line>,
    /// Current view into the buffer.
    /// Denotes which line is at the top of the screen.
    view_start: usize,
    /// Position of the cursor within the `ScreenBuffer`.
    cursor_pos: Position,
    /// Start of text selection. Used for highlighting and copying to clipboard.
    selection_start: Option<(u16, usize)>,
    /// End of text selection. Used for highlighting and copying to clipboard.
    selection_end: Option<(u16, usize)>,
    /// Configuration for the maximum amount of lines to keep in memory.
    max_scrollback: usize,
    /// Represents the current state for handling ansii escape sequences
    /// as incoming data is being processed.
    escape_state: EscapeState,
    /// As ascii escape sequences are recieved, they are built in the
    /// [`EscapeSequence`] to evaluate upon a completed escape sequence.
    escape_sequence: EscapeSequence,
    /// Represents the time since [`ScreenBuffer::render()`] was last called.
    last_render: Option<tokio::time::Instant>,
    display_attributes: Attributes,
    /// Indicates that [`ScreenBuffer`] has new data and needs to render.
    needs_render: bool,
}

impl ScreenBuffer {
    /// Constructs a new `ScreenBuffer`.
    ///
    /// Takes the `width` and `height` of the terminal.
    #[must_use]
    pub fn new(width: u16, height: u16) -> Self {
        let mut buffer = Self {
            width,
            height,
            lines: VecDeque::new(),
            view_start: 0,
            cursor_pos: Position::home(),
            selection_start: None,
            selection_end: None,
            max_scrollback: MAX_SCROLLBACK,
            last_render: None,
            needs_render: false,
            escape_state: EscapeState::Normal,
            escape_sequence: EscapeSequence::new(),
            display_attributes: Attributes::none(),
        };
        // Start with an empty line
        buffer.lines.push_back(Line::new_default(width as usize));
        buffer
    }

    fn set_char_at_cursor(&mut self, ch: char) {
        while self.cursor_pos.y >= self.lines.len() {
            self.lines.push_back(Line::new_default(self.width as usize));
        }

        if let Some(line) = self.lines.get_mut(self.cursor_pos.y)
            && (self.cursor_pos.x as usize) < line.len()
        {
            line.set_char(self.cursor_pos.x as usize, ch);
        }
    }

    fn clear_from_cursor_to_sol(&mut self) {
        if let Some(line) = self.lines.get_mut(self.cursor_pos.y) {
            line.reset_to(self.cursor_pos.x as usize);
        }
    }

    fn clear_from_cursor_to_sos(&mut self) {
        self.clear_from_cursor_to_sol();
        for line in self.lines.range_mut(self.view_start..self.cursor_pos.y) {
            line.reset();
        }
    }

    fn clear_from_cursor_to_eol(&mut self) {
        if let Some(line) = self.lines.get_mut(self.cursor_pos.y) {
            line.reset_from(self.cursor_pos.x as usize);
        }
    }

    fn clear_from_cursor_to_eos(&mut self) {
        self.clear_from_cursor_to_eol();
        for line in self.lines.range_mut(self.cursor_pos.y + 1..) {
            line.reset();
        }
    }

    fn clear_whole_line(&mut self) {
        if let Some(line) = self.lines.get_mut(self.cursor_pos.y) {
            line.reset();
        }
    }

    fn new_line(&mut self) {
        self.set_cursor_pos((0, self.cursor_pos.y + 1));

        if self.cursor_pos.y >= self.lines.len() {
            self.lines.push_back(Line::new_default(self.width as usize));
        }

        // Remove old lines if exceeding `ScreenBuffer.max_scrollback`
        while self.lines.len() > self.max_scrollback {
            self.lines.pop_front();
            // Update the view position
            if self.cursor_pos.y > 0 {
                self.cursor_pos.y -= 1;
            }
            if self.view_start > 0 {
                self.view_start -= 1;
            }
        }
    }
}
