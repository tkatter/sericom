use crossterm::style::Attributes;
use std::collections::VecDeque;

use super::position::{Position, TermPos, TranslatePos};
use super::{Line, Rect};

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
    pub(crate) cursor: Position<TermPos>,
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
    #[must_use]
    pub fn new(rect: Rect<TermPos>) -> Self {
        let mut buffer = Self {
            lines: VecDeque::new(),
            view_start: 0,
            rect,
            cursor: Position::ORIGIN,
            selection_start: None,
            selection_end: None,
            max_scrollback: MAX_SCROLLBACK,
        };
        // Start with an empty line
        buffer.lines.push_back(Line::reserve_new(rect.width.into()));
        buffer
    }

    pub(crate) const fn width(&self) -> u16 {
        self.rect.width
    }

    pub(crate) fn push_line(&mut self, line: Line) {
        self.lines.push_back(line);
    }

    pub fn save_cursor_pos<W: std::io::Write>(&mut self, stdout: &mut W) {
        use crossterm::cursor::SavePosition;
        crossterm::execute!(stdout, SavePosition);
    }

    pub fn restore_cursor_pos<W: std::io::Write>(&mut self, stdout: &mut W) {
        use crossterm::cursor::RestorePosition;
        crossterm::execute!(stdout, RestorePosition);
    }

    pub(crate) fn handle_span_colors(&mut self, colors: &super::ColorState, attrs: Attributes) {
        let curr_col = self.cursor.x as usize;
        let width = self.width() as usize;

        let line = self.curr_line_mut();
        let remainder = width - line.num_cells();

        line.split_spans(colors, attrs, curr_col, remainder);
    }

    pub(crate) fn with_current_span<F: FnOnce(&mut super::Span)>(&mut self, f: F) {
        let buff_pos = self.to_buff(self.cursor);

        let span = {
            let line = self.curr_line_mut();
            let (span_idx, _col_offset) = line.span_at_col(buff_pos.x as usize);
            line.get_mut_span(span_idx).expect("verified")
        };

        f(span);
    }

    pub(crate) fn with_current_line<F: FnOnce(&mut super::Line)>(&mut self, f: F) {
        let buff_pos = self.to_buff(self.cursor);

        let line = self.curr_line_mut();

        f(line);
    }

    pub(crate) fn curr_line(&mut self) -> &Line {
        let pos_in_lines = self.to_buff(self.cursor);
        if self.lines.get(pos_in_lines.y as usize).is_some() {
            self.lines
                .get(pos_in_lines.y as usize)
                .expect("verified that line exists")
        } else {
            self.push_line(Line::reserve_new(self.width() as usize));
            self.lines.back().expect("is not empty")
        }
    }

    pub(crate) fn curr_line_mut(&mut self) -> &mut Line {
        let pos_in_lines = self.to_buff(self.cursor);
        if self.lines.get(pos_in_lines.y as usize).is_some() {
            self.lines
                .get_mut(pos_in_lines.y as usize)
                .expect("verified that line exists")
        } else {
            self.push_line(Line::reserve_new(self.width() as usize));
            self.lines.back_mut().expect("is not empty")
        }
    }
    // fn clear_from_cursor_to_sol(&mut self) {
    //     if let Some(line) = self.lines.get_mut(usize::from(self.cursor.y)) {
    //         // line.reset_to(self.cursor.x as usize);
    //     }
    // }

    // fn clear_from_cursor_to_sos(&mut self) {
    //     self.clear_from_cursor_to_sol();
    //     for line in self
    //         .lines
    //         .range_mut(self.view_start..usize::from(self.cursor.y))
    //     {
    //         line.reset();
    //     }
    // }

    // fn clear_from_cursor_to_eol(&mut self) {
    //     if let Some(line) = self.lines.get_mut(usize::from(self.cursor.y)) {
    //         // line.reset_from(self.cursor.x as usize);
    //     }
    // }

    // fn clear_from_cursor_to_eos(&mut self) {
    //     self.clear_from_cursor_to_eol();
    //     for line in self.lines.range_mut(usize::from(self.cursor.y) + 1..) {
    //         line.reset();
    //     }
    // }

    // fn clear_whole_line(&mut self) {
    //     if let Some(line) = self.lines.get_mut(usize::from(self.cursor.y)) {
    //         line.reset();
    //     }
    // }

    // fn new_line(&mut self) {
    //     // TODO: F*X THIS
    //     // self.set_cursor_pos((0, self.cursor.y + 1));
    //
    //     if usize::from(self.cursor.y) >= self.lines.len() {
    //         self.lines
    //             .push_back(Line::new_default(usize::from(self.width())));
    //     }
    //
    //     // Remove old lines if exceeding `ScreenBuffer.max_scrollback`
    //     while self.lines.len() > self.max_scrollback {
    //         self.lines.pop_front();
    //         // Update the view position
    //         if self.cursor.y > 0 {
    //             self.cursor.y -= 1;
    //         }
    //         if self.view_start > 0 {
    //             self.view_start -= 1;
    //         }
    //     }
    // }

    // fn set_char_at_cursor(&mut self, ch: char) {
    //     while usize::from(self.cursor.y) >= self.lines.len() {
    //         self.lines
    //             .push_back(Line::new_default(usize::from(self.width())));
    //     }
    //
    //     if let Some(line) = self.lines.get_mut(usize::from(self.cursor.y))
    //         && (self.cursor.x as usize) < line.len()
    //     {
    //         // line.set_char(self.cursor.x as usize, ch);
    //     }
    // }

    // pub(crate) fn line_from_cursor(&mut self) -> usize {
    //     let line_idx = self.view_start + usize::from(self.cursor.y);
    //     while line_idx > self.lines.len() {
    //         self.lines
    //             .push_back(Line::new_empty(usize::from(self.rect.width)));
    //     }
    //     line_idx
    // }
}
