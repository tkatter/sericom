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

mod cursor;
mod escape;
mod render;
mod ui_command;
pub use ui_command::*;

use crate::ui::{Cursor, Line, Rect};
use std::collections::VecDeque;

/// The maximum number of lines stored in memory in [`ScreenBuffer`].
pub const MAX_SCROLLBACK: usize = 10000;

/// The `ScreenBuffer` holds rendering state for the entire terminal's window/frame.
///
/// It mainly serves to allow for user-interactions that require a history and location
/// of the data displayed within the terminal i.e. copy/paste, scrolling, & highlighting.
#[derive(Debug)]
pub struct ScreenBuffer {
    /// Scrollback buffer (all lines received from the serial connection).
    /// Limited by memory.
    lines: VecDeque<Line>,
    /// Current view into the buffer.
    /// Denotes which line is at the top of the screen.
    view_start: usize,
    /// The terminal's dimensions
    rect: Rect,
    /// Position of the cursor within the `ScreenBuffer`.
    cursor: crate::ui::Position,
    /// Start of text selection. Used for highlighting and copying to clipboard.
    selection_start: Option<(u16, usize)>,
    /// End of text selection. Used for highlighting and copying to clipboard.
    selection_end: Option<(u16, usize)>,
    /// Configuration for the maximum amount of lines to keep in memory.
    max_scrollback: usize,
}

impl ScreenBuffer {
    /// Constructs a new `ScreenBuffer`.
    ///
    /// Takes the `width` and `height` of the terminal.
    pub fn new(rect: Rect) -> Self {
        let mut buffer = Self {
            lines: VecDeque::new(),
            view_start: 0,
            rect,
            cursor: crate::ui::Position::ORIGIN,
            selection_start: None,
            selection_end: None,
            max_scrollback: MAX_SCROLLBACK,
        };
        // Start with an empty line
        buffer.lines.push_back(Line::new_default(rect.width.into()));
        buffer
    }

    fn width(&self) -> u16 {
        self.rect.width
    }

    fn set_char_at_cursor(&mut self, ch: char) {
        while usize::from(self.cursor.y) >= self.lines.len() {
            self.lines
                .push_back(Line::new_default(usize::from(self.width())));
        }

        if let Some(line) = self.lines.get_mut(usize::from(self.cursor.y))
            && (self.cursor.x as usize) < line.len()
        {
            line.set_char(self.cursor.x as usize, ch);
        }
    }

    fn clear_from_cursor_to_sol(&mut self) {
        if let Some(line) = self.lines.get_mut(usize::from(self.cursor.y)) {
            line.reset_to(self.cursor.x as usize);
        }
    }

    fn clear_from_cursor_to_sos(&mut self) {
        self.clear_from_cursor_to_sol();
        for line in self
            .lines
            .range_mut(self.view_start..usize::from(self.cursor.y))
        {
            line.reset();
        }
    }

    fn clear_from_cursor_to_eol(&mut self) {
        if let Some(line) = self.lines.get_mut(usize::from(self.cursor.y)) {
            line.reset_from(self.cursor.x as usize);
        }
    }

    fn clear_from_cursor_to_eos(&mut self) {
        self.clear_from_cursor_to_eol();
        for line in self.lines.range_mut(usize::from(self.cursor.y) + 1..) {
            line.reset();
        }
    }

    fn clear_whole_line(&mut self) {
        if let Some(line) = self.lines.get_mut(usize::from(self.cursor.y)) {
            line.reset();
        }
    }

    fn new_line(&mut self) {
        self.set_cursor_pos((0, self.cursor.y + 1));

        if usize::from(self.cursor.y) >= self.lines.len() {
            self.lines
                .push_back(Line::new_default(usize::from(self.width())));
        }

        // Remove old lines if exceeding `ScreenBuffer.max_scrollback`
        while self.lines.len() > self.max_scrollback {
            self.lines.pop_front();
            // Update the view position
            if self.cursor.y > 0 {
                self.cursor.y -= 1;
            }
            if self.view_start > 0 {
                self.view_start -= 1;
            }
        }
    }

    pub(crate) fn push_line(&mut self, curr_line: Line) {
        self.lines.push_back(curr_line);
    }
}

impl crate::ui::Cursor for ScreenBuffer {
    /// Sets the cursor position.
    fn set_cursor_pos<P: Into<crate::ui::Position>>(&mut self, position: P) {
        self.cursor = position.into();
    }

    /// Moves the cursor left by `cells`.
    fn move_cursor_left(&mut self, cells: u16) {
        self.cursor.x = self.cursor.x.saturating_sub(cells);
    }

    /// Moves the cursor up by `lines`.
    fn move_cursor_up(&mut self, lines: u16) {
        self.cursor.y = self.cursor.y.saturating_sub(lines);
    }

    /// Moves the cursor down by `lines`.
    fn move_cursor_down(&mut self, lines: u16) {
        self.cursor.y = self.cursor.y.saturating_add(lines);
        while usize::from(self.cursor.y) > self.lines.len() {
            self.lines
                .push_back(Line::new_default(usize::from(self.width())));
        }
    }

    /// Moves the cursor right by `cells`.
    fn move_cursor_right(&mut self, cells: u16) {
        self.cursor.x = self.cursor.x.saturating_add(cells);
    }

    /// Sets the column of the cursor
    fn set_cursor_col(&mut self, col: u16) {
        self.cursor.x = col;
    }
}
