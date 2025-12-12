use std::fmt::Write;

use crossterm::style::Attributes;

use crate::screen::process::SEP;

use super::ScreenBuffer;
use super::components::{Cell, Line};
use super::position::Cursor;
use super::process::{BK, BS, CR, ColorState, ESC, FF, NL, ParserEvent, TAB};
use super::{process_colors, process_cursor, process_erase};

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

    pub fn process_events(&mut self, events: Vec<ParserEvent>) {
        for event in events {
            tracing::trace!(target: "parser::events", %event);
            match event {
                ParserEvent::Text(bytes) => self.write_text(&bytes),
                ParserEvent::Control(ctrl) => self.handle_control(ctrl),
                ParserEvent::EscapeSequence(seq) => self.handle_escape(&seq),
            }
        }
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

    fn handle_control(&mut self, ctrl: u8) {
        match ctrl {
            BS => self.buffer.move_cursor_left(1),
            NL => {
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
                    tracing::trace!(target: "parser::newline", ?line);
                });
                self.buffer.set_cursor_col(0);
                self.buffer.move_cursor_down(1);
                self.buffer
                    .push_line(Line::new_empty(self.buffer.width() as usize));
            }
            TAB => {
                self.buffer.with_current_span(|span, _| {
                    span.push(Cell::TAB);
                });
                self.buffer.cursor.tab();
            }
            CR => self.buffer.set_cursor_col(0),
            // Not sure that FF needs to be handled
            #[allow(clippy::match_same_arms)]
            FF => {}
            _ => {}
        }
    }

    fn handle_escape(&mut self, seq: &[u8]) {
        let Some(seq_type) = classify_escape_seq(seq) else {
            let s = seq.iter().fold(String::new(), |mut output, b| {
                if *b == ESC {
                    let _ = write!(output, "ESC");
                } else if *b == BK {
                    let _ = write!(output, "[");
                } else if *b == SEP {
                    let _ = write!(output, ";");
                }
                let _ = write!(output, "{}", *b as char);
                output
            });
            tracing::debug!(target: "parser::escape", sequence=%s, "Failed to classify escape sequence");
            return;
        };
        match seq_type {
            EscSequenceType::Cursor(kind) => {
                process_cursor(seq, kind, self.buffer);
            }
            EscSequenceType::Erase(kind) => process_erase(seq, kind, self.buffer),
            EscSequenceType::Graphics => {
                process_colors(seq, &mut self.color_state, &mut self.attrs);
                self.buffer
                    .handle_span_colors(&self.color_state, self.attrs);
            }
            EscSequenceType::Screen(_kind) => { /* process_screen(seq, kind, self.buffer) */ }
        }
    }
}

/// The category of escape sequence determined by the last char of the sequence.
pub enum EscSequenceType {
    /// Control cursor movement
    Cursor(u8),
    /// Sequences that clear the screen
    Erase(u8),
    /// Color changes/modes
    Graphics,
    /// Set screen modes
    Screen(u8),
}

pub fn classify_escape_seq(seq: &[u8]) -> Option<EscSequenceType> {
    // https://gist.github.com/fnky/458719343aabd01cfb17a3a4f7296797
    // Ensures the sequence resembles: ESC[<sequence>
    if seq.len() < 3 || seq[0] != ESC || seq[1] != BK {
        return None;
    }

    let last = *seq.last().expect("Verified len != 0");
    match last {
        b'm' => Some(EscSequenceType::Graphics),
        b'A'..=b'G' | b'H' | b'f' | b'n' | b's' | b'u' => Some(EscSequenceType::Cursor(last)),
        b'J' | b'K' => Some(EscSequenceType::Erase(last)),
        b'h' | b'l' => Some(EscSequenceType::Screen(last)),
        _ => None,
    }
}
