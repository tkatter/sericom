use crossterm::style::Attributes;
use tracing::{debug, trace};

use super::{C0, C1, CSI, ColorState, CsiKind, ParserEvent, SEMI, digits_to_int, process_colors};
use crate::screen::{
    Cell, Cursor as _, Line, ScreenBuffer,
    process::{EDKind, ELKind},
};

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
    pub fn process_events(&mut self, events: Vec<ParserEvent<'_>>) -> u32 {
        let span = tracing::trace_span!("process");
        let _enter = span.enter();

        let mut newlines = 0;
        for event in events {
            trace!(%event);
            match event {
                ParserEvent::Text(bytes) => self.write_text(bytes),
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
                self.buffer.move_cursor_down(1);
                self.buffer.set_cursor_col(1);
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
            C0::CR => self.buffer.set_cursor_col(1),
            other => debug!("recieved unsupported C0: {:?}", other),
        }
    }

    #[allow(clippy::too_many_lines)]
    fn handle_csi(&mut self, csi: CSI) {
        match csi.kind {
            CsiKind::CursorUp => {
                let int = digits_to_int(csi.params).unwrap_or(1);
                self.buffer.move_cursor_up(int);
            }
            CsiKind::CursorDown | CsiKind::LineRel => {
                let int = digits_to_int(csi.params).unwrap_or(1);
                self.buffer.move_cursor_down(int);
            }
            CsiKind::CursorForward | CsiKind::CharRel => {
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
            CsiKind::CharAbsCHA | CsiKind::CharAbsHPA => {
                let int = digits_to_int(csi.params).unwrap_or(1);
                self.buffer.set_cursor_col(int);
            }
            CsiKind::PositionCUP | CsiKind::PositionHVP => {
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
            CsiKind::EraseInDisplay => self
                .buffer
                .erase_in_display(EDKind::try_from(csi.params).unwrap_or_default()),
            CsiKind::EraseInLine => self
                .buffer
                .erase_in_line(ELKind::try_from(csi.params).unwrap_or_default()),
            CsiKind::InsertLine => {
                let int = digits_to_int(csi.params).unwrap_or(1);
                self.buffer.insert_lines(int);
            }
            CsiKind::DeleteLine => {
                let int = digits_to_int(csi.params).unwrap_or(1);
                self.buffer.delete_lines(int);
            }
            CsiKind::DeleteChars => {
                let int = digits_to_int(csi.params).unwrap_or(1);
                self.buffer.delete_chars(int);
            }
            CsiKind::ScrollUp => {
                let int = digits_to_int(csi.params).unwrap_or(1);
                self.buffer.scroll_up(int);
            }
            CsiKind::ScrollDown | CsiKind::ScrollDown1991 => {
                let int = digits_to_int(csi.params).unwrap_or(1);
                self.buffer.scroll_down(int);
            }
            CsiKind::EraseChars => {
                let int = digits_to_int(csi.params).unwrap_or(1);
                self.buffer.erase_chars(int);
            }
            CsiKind::BackTab => {
                let int = digits_to_int(csi.params).unwrap_or(1);
                self.buffer.rtab(int);
            }
            CsiKind::Rep => {
                let int = digits_to_int(csi.params).unwrap_or(1);
                self.buffer.with_current_line(|line, cursor| {
                    let last_char = {
                        // if last_char is intended to be ' ' then this will break
                        let i = line.last_filled_idx();
                        line.iter()
                            .flatten()
                            .nth(i)
                            .expect("idx is always a cell")
                            .character
                    };
                    line.iter_mut()
                        .flatten()
                        .skip(usize::from(cursor.x))
                        .take(usize::from(int))
                        .for_each(|c| c.character = last_char);
                });
            }
            CsiKind::LineAbs => {
                let int = digits_to_int(csi.params).unwrap_or(1);
                self.buffer.set_cursor_row(int);
            }
            CsiKind::SGR => {
                process_colors(csi.params, &mut self.color_state, &mut self.attrs);
                self.buffer
                    .handle_span_colors(&self.color_state, self.attrs);
            }
        }
    }

    fn handle_c1(&mut self, c1: C1) {
        match c1 {
            C1::IND | C1::DECFI => self.buffer.move_cursor_down(1),
            C1::RI | C1::DECBI => self.buffer.move_cursor_up(1),
            C1::NEL => {
                self.buffer.move_cursor_down(1);
                self.buffer.set_cursor_col(1);
            }
            C1::RIS => self.buffer.erase_in_display(EDKind::All),
            other => debug!("recieved unsupported C1: {:?}", other),
        }
    }
}
