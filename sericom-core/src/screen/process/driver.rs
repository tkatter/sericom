use crossterm::style::Attributes;
use tracing::trace;

use super::{C0, C1, CSI, ColorState, CsiKind, ParserEvent, SEMI, digits_to_int, process_colors};
use crate::screen::{Cell, Cursor as _, Line, ScreenBuffer};

/// The layer between incoming [`ParserEvent`]s and the [`ScreenBuffer`].
pub struct ScreenDriver<'a> {
    buffer: &'a mut ScreenBuffer,
    color_state: ColorState,
    attrs: Attributes,
}

impl<'a> ScreenDriver<'a> {
    pub fn new(buffer: &'a mut ScreenBuffer) -> Self {
        Self {
            buffer,
            color_state: ColorState::default(),
            attrs: Attributes::default(),
        }
    }

    /// Returns the number of newlines written
    pub fn process_events<'b>(&mut self, events: Vec<ParserEvent<'b>>) -> u32 {
        let span = tracing::trace_span!("process");
        let _enter = span.enter();

        let mut newlines = 0;
        for event in events {
            trace!(%event);
            match event {
                ParserEvent::Text(bytes) => self.write_text(&bytes),
                ParserEvent::CSI(csi) => self.handle_csi(csi),
                ParserEvent::C1(c1) => self.handle_c1(c1),
                ParserEvent::C0(c0) => {
                    if c0 == C0::NL {
                        newlines += 1;
                    }
                    self.handle_c0(c0);
                }
            }
        }
        newlines
    }

    fn write_text(&mut self, bytes: &[u8]) {
        self.buffer.with_current_span(|span, offset| {
            for (cell, ch) in span.cells.iter_mut().skip(offset).zip(bytes) {
                cell.character = *ch;
            }
        });

        // Truncating is unlikely to happen in this scenario, but even so,
        // `move_cursor_right` will clamp to `self.width` so its ok.
        #[allow(clippy::cast_possible_truncation)]
        self.buffer.move_cursor_right(bytes.len() as u16);
    }

    fn handle_c0(&mut self, c0: C0) {
        match c0 {
            C0::BS => self.buffer.move_cursor_left(1),
            C0::NL => {
                self.buffer.with_current_line(|line, _| {
                    // pushing to the end of line unconditionally because
                    // NL is always the end of a line, and if received a CR
                    // before NL, then `with_current_span` would behave incorrect
                    if let Some(span) = line.0.last_mut() {
                        let last = span.last_filled_idx();
                        span.cells
                            .get_mut(last)
                            .expect("span len is greater than last filled cell")
                            .character = b'\n';
                    }
                });
                self.buffer.set_cursor_col(0);
                self.buffer.move_cursor_down(1);
                self.buffer
                    .push_line(Line::new_empty(self.buffer.width() as usize));
                self.buffer.update_view(None);
            }
            C0::HT => {
                self.buffer.with_current_span(|span, _| {
                    span.push(Cell::TAB);
                });
                self.buffer.cursor.tab();
            }
            C0::CR => self.buffer.set_cursor_col(0),
            _ => {}
        }
    }

    fn handle_csi(&mut self, csi: CSI) {
        match csi.kind {
            CsiKind::CursorUp => {
                let int = digits_to_int(csi.params).unwrap_or(1);
                self.buffer.move_cursor_up(int);
            }
            CsiKind::CursorDown => {
                let int = digits_to_int(csi.params).unwrap_or(1);
                self.buffer.move_cursor_down(int);
            }
            CsiKind::CursorForward => {
                let int = digits_to_int(csi.params).unwrap_or(1);
                self.buffer.move_cursor_right(int);
            }
            CsiKind::CursorBackward => {
                let int = digits_to_int(csi.params).unwrap_or(1);
                self.buffer.move_cursor_left(int);
            }
            CsiKind::NextLine => {
                let int = digits_to_int(csi.params).unwrap_or(1);
                self.buffer.move_cursor_down(int);
                self.buffer.set_cursor_col(0);
            }
            CsiKind::PrecedingLine => {
                let int = digits_to_int(csi.params).unwrap_or(1);
                self.buffer.move_cursor_up(int);
                self.buffer.set_cursor_col(0);
            }
            CsiKind::CharAbsCHA => {
                let int = digits_to_int(csi.params).unwrap_or(1);
                self.buffer.set_cursor_col(int);
            }
            CsiKind::PositionCUP => {
                if csi.params.is_empty() {
                    self.buffer.set_cursor_pos((1u16, 1u16));
                    return;
                }
                if let Some(idx) = csi.params.iter().position(|b| *b == SEMI) {
                    let (row, col) = (
                        digits_to_int(&csi.params[0..idx]).unwrap_or(1),
                        digits_to_int(&csi.params[idx + 1..]).unwrap_or(1),
                    );
                    self.buffer.set_cursor_pos((col, row));
                } else {
                    self.buffer.set_cursor_pos((1u16, 1u16));
                }
            }
            CsiKind::ForwardTab => {
                let int = digits_to_int(csi.params).unwrap_or(1);
                self.buffer.tab(int);
            }
            CsiKind::EraseScreen => todo!(),
            CsiKind::EraseLine => todo!(),
            CsiKind::InsertLine => todo!(),
            CsiKind::DeleteLine => todo!(),
            CsiKind::DeleteChars => todo!(),
            CsiKind::ScrollUp => todo!(),
            CsiKind::ScrollDown => todo!(),
            CsiKind::EraseChars => todo!(),
            CsiKind::BackTab => {
                let int = digits_to_int(csi.params).unwrap_or(1);
                self.buffer.rtab(int);
            }
            CsiKind::CharRel => todo!(),
            CsiKind::Rep => todo!(),
            CsiKind::LineAbs => todo!(),
            CsiKind::LineRel => todo!(),
            CsiKind::PositionHVP => todo!(),
            CsiKind::CharAbsHPA => todo!(),
            CsiKind::Sgr => {
                process_colors(csi.params, &mut self.color_state, &mut self.attrs);
                self.buffer
                    .handle_span_colors(&self.color_state, self.attrs);
            }
            CsiKind::ScrollDown1991 => todo!(),
        }
    }

    fn handle_c1(&mut self, c1: C1) {}
}
