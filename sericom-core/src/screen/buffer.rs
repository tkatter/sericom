use crossterm::style::Attributes;
use std::collections::VecDeque;

use crate::screen::process::{EDKind, ELKind};
use crate::screen::{BuffPos, Cell, UICommand};

use super::position::{Position, TermPos, TranslatePos};
use super::{Line, Rect};

/// The maximum number of lines stored in memory in [`ScreenBuffer`].
pub const MAX_SCROLLBACK: u32 = 10000;

/// The `ScreenBuffer` holds rendering state for the entire terminal's window/frame.
///
/// It mainly serves to allow for user-interactions that require a history and location
/// of the data displayed within the terminal i.e. copy/paste, scrolling, & highlighting.
#[derive(Debug)]
pub struct ScreenBuffer {
    /// Scrollback buffer (all lines received from the serial connection).
    /// Limited by [`MAX_SCROLLBACK`].
    pub(crate) lines: VecDeque<Line>,
    /// Denotes which line (as idx in [`Self::lines`]) is at the top of the screen.
    ///
    /// [`Self::Lines`]: ScreenBuffer::lines
    pub(crate) view_start: u32,
    /// The terminal's dimensions
    pub(crate) rect: Rect<TermPos>,
    /// Position of the cursor within the `ScreenBuffer`.
    pub(crate) cursor: Position<TermPos>,
    /// Start of text selection. Used for highlighting and copying to clipboard.
    selection_start: Option<(u16, u32)>,
    /// End of text selection. Used for highlighting and copying to clipboard.
    selection_end: Option<(u16, u32)>,
    /// Configuration for the maximum amount of lines to keep in memory.
    max_scrollback: u32,
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
        let curr_col = {
            let c = self.to_buff(&self.cursor);
            c.x.into()
        };

        let line = self.curr_line_mut();
        line.split_spans(colors, attrs, curr_col);
    }

    pub(crate) fn with_current_span<F: FnOnce(&mut super::Span, usize)>(&mut self, f: F) {
        let buff_pos = self.to_buff(&self.cursor);

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
        let buff_pos = self.to_buff(&self.cursor);
        let line = self.curr_line_mut();

        f(line, &buff_pos);
    }

    pub(crate) fn curr_line(&self) -> Option<&Line> {
        let pos_in_lines = self.to_buff(&self.cursor);
        self.lines.get(pos_in_lines.y as usize)
    }

    pub(crate) fn curr_line_mut(&mut self) -> &mut Line {
        let pos_in_lines = self.to_buff(&self.cursor);
        if self.lines.get(pos_in_lines.y as usize).is_some() {
            self.lines
                .get_mut(pos_in_lines.y as usize)
                .expect("verified that line exists")
        } else {
            for _ in self.lines.len()..=pos_in_lines.y as usize {
                self.push_line(Line::new_empty(self.width() as usize));
            }
            self.lines
                .get_mut(pos_in_lines.y as usize)
                .expect("is not empty")
        }
    }

    pub(crate) fn buff_rect(&self) -> Rect<BuffPos> {
        Rect::from((
            (0_u16, self.view_start),
            self.width(),
            u32::from(self.height()),
        ))
    }

    #[allow(clippy::cast_possible_truncation)]
    #[tracing::instrument(skip_all, level = "trace", name = "update_view")]
    pub(crate) fn update_view(&mut self, update: Option<UICommand>) {
        let buff_rect = self.buff_rect();
        let num_lines = self.lines.len();

        let Some(_cmd) = update else {
            // Return because still filling up the initial/empty screen 0..TERM_HEIGHT
            if num_lines <= (self.view_start + buff_rect.height) as usize {
                return;
            }

            let additional = (num_lines as u32 - (self.view_start + buff_rect.height)).max(1);
            self.view_start += additional;
            return;
        };

        todo!();
    }

    pub(crate) fn insert_lines(&mut self, num: u16) {
        let buff_pos = self.to_buff(&self.cursor);
        for _ in 0..num {
            self.lines.insert(
                buff_pos.y as usize,
                Line::new_empty(usize::from(self.width())),
            );
            if self.lines.len() >= self.buff_rect().bottom() as usize {
                self.lines.pop_back();
            }
        }
        self.cursor.x = 1;
    }

    pub(crate) fn delete_lines(&mut self, num: u16) {
        let buff_pos = self.to_buff(&self.cursor);
        for _ in 0..num {
            self.lines.remove(buff_pos.y as usize);
            self.lines
                .push_back(Line::new_empty(usize::from(self.width())));
        }
        self.cursor.x = 1;
    }

    #[allow(clippy::cast_possible_truncation)]
    pub(crate) fn erase_in_display(&mut self, kind: EDKind) {
        match kind {
            EDKind::Below => {
                let buff_range = std::ops::Range {
                    start: (self.to_buff(&self.cursor).y + 1) as usize,
                    end: (self.buff_rect().bottom() + 1) as usize,
                };

                self.clear_line_from_cursor();
                self.clear_lines(buff_range);
            }
            EDKind::Above => {
                let buff_range = std::ops::Range {
                    start: self.view_start as usize,
                    end: self.to_buff(&self.cursor).y as usize,
                };

                self.clear_lines(buff_range);
                self.clear_line_to_cursor();
            }
            _ => {
                self.lines
                    .push_back(Line::new_empty(usize::from(self.width())));
                // Casting to u32 from usize is fine because self.lines should never
                // exceed MAX_SCROLLBACK which is < usize::MAX
                self.view_start = (self.lines.len() as u32).saturating_sub(1);
                self.cursor = Position::<TermPos>::ORIGIN;
            }
        }
    }

    pub(crate) fn erase_in_line(&mut self, kind: ELKind) {
        match kind {
            ELKind::Right => {
                self.clear_line_from_cursor();
            }
            ELKind::Left => self.clear_line_to_cursor(),
            ELKind::All => {
                self.with_current_line(|line, _| {
                    line.iter_mut().flatten().for_each(|cell| {
                        cell.character = b' ';
                    });
                });
            }
        }
    }

    pub(crate) fn erase_chars(&mut self, num: u16) {
        self.with_current_line(|line, cursor| {
            line.iter_mut()
                .flatten()
                .skip(usize::from(cursor.x))
                .take(usize::from(num))
                .for_each(|c| c.character = b' ');
        });
    }

    pub(crate) fn delete_chars(&mut self, num: u16) {
        self.with_current_line(|line, cursor| {
            let mut ttl_del = 0;

            if line.0.len() == 1 {
                if let Some(span) = line.get_mut_span(0) {
                    ttl_del += span
                        .cells
                        .drain(usize::from(cursor.x)..usize::from(cursor.x + num))
                        .len();
                    span.cells.resize(span.len() + ttl_del, Cell::EMPTY);
                }
            } else {
                let (start_span, start_off) = line.span_at_col(usize::from(cursor.x));
                let (end_span, end_off) = line.span_at_col(usize::from(cursor.x + num));

                if start_span == end_span {
                    if let Some(span) = line.get_mut_span(start_span) {
                        ttl_del += span
                            .cells
                            .drain(usize::from(cursor.x)..usize::from(cursor.x + num))
                            .len();
                        span.cells.shrink_to(span.len());
                    }

                    if let Some(span) = line.get_mut_span(start_span + 1) {
                        span.cells.resize(span.len() + ttl_del, Cell::EMPTY);
                    }
                } else if end_span - start_span == 1 {
                    if let Some(span) = line.get_mut_span(start_span) {
                        ttl_del += span.cells.drain(start_off..).len();
                        span.cells.shrink_to(start_off);
                    }

                    if let Some(span) = line.get_mut_span(end_span) {
                        ttl_del += span.cells.drain(..end_off).len();
                        span.cells.resize(span.len() + ttl_del, Cell::EMPTY);
                    }
                } else {
                    // remove any spans between start and end span
                    if let Some(span) = line.get_mut_span(start_span) {
                        ttl_del += span.cells.drain(start_off..).len();
                        span.cells.shrink_to(start_off);
                    }

                    for span_between in start_span + 1..end_span {
                        ttl_del += line.0.remove(span_between).cells.len();
                    }

                    if let Some(span) = line.get_mut_span(start_span + 1) {
                        ttl_del += span.cells.drain(..end_off).len();
                        span.cells.resize(span.len() + ttl_del, Cell::EMPTY);
                    }
                }
            }
        });
    }

    fn clear_line_to_cursor(&mut self) {
        self.with_current_line(|line, cursor| {
            for (idx, cell) in line.iter_mut().flatten().enumerate() {
                if idx < usize::from(cursor.x) {
                    cell.character = b' ';
                }
            }
        });
    }

    fn clear_line_from_cursor(&mut self) {
        self.with_current_line(|line, cursor| {
            line.iter_mut()
                .flatten()
                .skip(usize::from(cursor.x))
                .for_each(|cell| {
                    cell.character = b' ';
                });
        });
    }

    fn clear_lines(&mut self, range: std::ops::Range<usize>) {
        for line in self.lines.range_mut(range) {
            line.iter_mut().flatten().for_each(|cell| {
                cell.character = b' ';
            });
        }
    }

    pub(crate) fn scroll_down(&mut self, num: u16) {
        for _ in 0..num {
            self.lines.insert(
                self.view_start as usize,
                Line::new_empty(usize::from(self.width())),
            );
            if self.lines.len() >= self.buff_rect().bottom() as usize {
                self.lines.pop_back();
            }
        }
    }

    pub(crate) fn scroll_up(&mut self, num: u16) {
        for _ in 0..num {
            self.view_start += 1;
            self.lines
                .push_back(Line::new_empty(usize::from(self.width())));
        }
    }
}
