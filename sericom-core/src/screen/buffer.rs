use crossterm::style::Attributes;
use std::collections::VecDeque;

use crate::screen::BuffPos;

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
        buffer.lines.push_back(Line::new_empty(rect.width.into()));
        buffer
    }

    pub(crate) const fn width(&self) -> u16 {
        self.rect.width
    }

    pub(crate) const fn height(&self) -> u16 {
        self.rect.height
    }

    pub(crate) fn push_line(&mut self, line: Line) {
        self.lines.push_back(line);
    }

    pub fn save_cursor_pos<W: std::io::Write>(&mut self, stdout: &mut W) {
        use crossterm::cursor::SavePosition;
        let _ = crossterm::execute!(stdout, SavePosition);
    }

    pub fn restore_cursor_pos<W: std::io::Write>(&mut self, stdout: &mut W) {
        use crossterm::cursor::RestorePosition;
        let _ = crossterm::execute!(stdout, RestorePosition);
    }

    pub(crate) fn handle_span_colors(&mut self, colors: &super::ColorState, attrs: Attributes) {
        let curr_col = self.cursor.x.into();

        let line = self.curr_line_mut();
        line.split_spans(colors, attrs, curr_col);
    }

    pub(crate) fn with_current_span<F: FnOnce(&mut super::Span, usize)>(&mut self, f: F) {
        let buff_pos = self.to_buff(self.cursor);

        let (span, offset) = {
            let line = self.curr_line_mut();
            let (span_idx, col_offset) = line.span_at_col(buff_pos.x as usize);
            (line.get_mut_span(span_idx).expect("verified"), col_offset)
        };

        f(span, offset);
    }

    pub(crate) fn with_current_line<F: FnOnce(&mut super::Line, &Position<BuffPos>)>(
        &mut self,
        f: F,
    ) {
        let buff_pos = self.to_buff(self.cursor);
        let line = self.curr_line_mut();

        f(line, &buff_pos);
    }

    pub(crate) fn curr_line(&self) -> Option<&Line> {
        let pos_in_lines = self.to_buff(self.cursor);
        self.lines.get(pos_in_lines.y as usize)
    }

    pub(crate) fn curr_line_mut(&mut self) -> &mut Line {
        let pos_in_lines = self.to_buff(self.cursor);
        if self.lines.get(pos_in_lines.y as usize).is_some() {
            self.lines
                .get_mut(pos_in_lines.y as usize)
                .expect("verified that line exists")
        } else {
            self.push_line(Line::new_empty(self.width() as usize));
            self.lines.back_mut().expect("is not empty")
        }
    }

    pub(crate) fn buff_rect(&self) -> Rect<BuffPos> {
        Rect::from((
            (
                0_u16,
                u32::try_from(self.view_start).expect("ScreenBuffer is less than usize::MAX"),
            ),
            self.width(),
            u32::from(self.height()),
        ))
    }
}
