use crate::ui::{self, Cursor, Line, Rect, TermPos};
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
    /// Limited by [`MAX_SCROLLBACK`].
    pub(crate) lines: VecDeque<Line>,
    /// Current view into the buffer.
    /// Denotes which line is at the top of the screen.
    pub(crate) view_start: usize,
    /// The terminal's dimensions
    pub(crate) rect: Rect<TermPos>,
    /// Position of the cursor within the `ScreenBuffer`.
    pub(crate) cursor: ui::Position<TermPos>,
    /// Start of text selection. Used for highlighting and copying to clipboard.
    selection_start: Option<(u16, usize)>,
    /// Saved cursor position from ascii escape sequence
    pub(crate) saved_cursor: Option<ui::Position<TermPos>>,
    /// End of text selection. Used for highlighting and copying to clipboard.
    selection_end: Option<(u16, usize)>,
    /// Configuration for the maximum amount of lines to keep in memory.
    max_scrollback: usize,
}

impl ScreenBuffer {
    /// Constructs a new `ScreenBuffer`.
    ///
    /// Takes the `width` and `height` of the terminal.
    pub fn new(rect: Rect<TermPos>) -> Self {
        let mut buffer = Self {
            lines: VecDeque::new(),
            view_start: 0,
            rect,
            cursor: ui::Position::ORIGIN,
            saved_cursor: None,
            selection_start: None,
            selection_end: None,
            max_scrollback: MAX_SCROLLBACK,
        };
        // Start with an empty line
        buffer.lines.push_back(Line::reserve_new(rect.width.into()));
        buffer
    }

    const fn width(&self) -> u16 {
        self.rect.width
    }

    pub(crate) fn cursor_is_at_end(&self) -> bool {
        if self.lines.is_empty() {
            return self.cursor.x == 0 && self.cursor.y == 0;
        }

        let bottom_window_line = self.view_start + usize::from(self.rect.height);
        let final_line = self.lines.len();
        let line_relative_pos = final_line - self.view_start;

        if final_line > bottom_window_line {
            false
        } else {
            let last_cell_pos = self
                .lines
                .get(final_line - 1)
                .map_or_else(|| 0, |line| line.last_cell_idx());
            self.cursor.x as usize == last_cell_pos && self.cursor.y as usize == line_relative_pos
        }
    }

    pub(crate) fn line_from_cursor(&mut self) -> usize {
        let line_idx = self.view_start + usize::from(self.cursor.y);
        while line_idx > self.lines.len() {
            self.lines
                .push_back(Line::new_empty(usize::from(self.rect.width)));
        }
        line_idx
    }

    fn set_char_at_cursor(&mut self, ch: char) {
        while usize::from(self.cursor.y) >= self.lines.len() {
            self.lines
                .push_back(Line::new_default(usize::from(self.width())));
        }

        if let Some(line) = self.lines.get_mut(usize::from(self.cursor.y))
            && (self.cursor.x as usize) < line.len()
        {
            // line.set_char(self.cursor.x as usize, ch);
        }
    }

    fn clear_from_cursor_to_sol(&mut self) {
        if let Some(line) = self.lines.get_mut(usize::from(self.cursor.y)) {
            // line.reset_to(self.cursor.x as usize);
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
            // line.reset_from(self.cursor.x as usize);
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
        // TODO: F*X THIS
        // self.set_cursor_pos((0, self.cursor.y + 1));

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

    pub const fn save_cursor_pos(&mut self) {
        self.saved_cursor = Some(self.cursor);
    }

    pub const fn restore_cursor_pos(&mut self) {
        if let Some(saved_cursor) = self.saved_cursor {
            self.cursor = saved_cursor;
            self.saved_cursor = None;
        }
    }
}
