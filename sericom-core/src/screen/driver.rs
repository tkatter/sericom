use crossterm::style::Attributes;
use tracing::Instrument;

use super::ScreenBuffer;
use super::components::{Cell, Line, Span};
use super::position::{Cursor, TranslatePos};
use super::process::{BK, BS, CR, ColorState, ESC, FF, NL, ParserEvent, TAB};
use super::{process_colors, process_cursor, process_erase, process_screen};

/// The layer between incoming [`ParserEvent`]s and the [`ScreenBuffer`].
pub struct ScreenDriver<'a, W: std::io::Write> {
    buffer: &'a mut ScreenBuffer,
    color_state: ColorState,
    attrs: Attributes,
    // stdout gives access to call crossterm::execute!/queue!
    stdout: &'a mut W,
}

impl<'a, W: std::io::Write> ScreenDriver<'a, W> {
    pub fn new(buffer: &'a mut ScreenBuffer, stdout: &'a mut W) -> Self {
        Self {
            buffer,
            color_state: ColorState::default(),
            attrs: Attributes::default(),
            stdout,
        }
    }

    pub fn process_events(&mut self, events: Vec<ParserEvent>) {
        for ev in events {
            eprintln!("{ev}");
            match ev {
                ParserEvent::Text(bytes) => self.write_text(&bytes),
                ParserEvent::Control(ctrl) => self.handle_control(ctrl),
                ParserEvent::EscapeSequence(seq) => self.handle_escape(&seq),
            }
        }
    }

    fn write_text(&mut self, bytes: &[u8]) {
        let chars: Vec<char> = bytes.iter().map(|b| char::from(*b)).collect();
        let num_chars = chars.len();

        self.buffer.with_current_span(|span, cursor| {
            if !span.is_empty() {
                span.cells.truncate(cursor.x.into());
            }

            for ch in chars {
                span.push(Cell::new(ch));
            }
        });

        // Truncating is unlikely to happen in this scenario, but even so,
        // `move_cursor_right` will clamp to `self.width` so its ok.
        #[allow(clippy::cast_possible_truncation)]
        self.buffer.move_cursor_right(num_chars as u16);
    }

    fn handle_control(&mut self, ctrl: u8) {
        match ctrl {
            BS => self.buffer.move_cursor_left(1),
            NL => {
                // #[cfg(feature = "cli")]
                // {
                self.buffer.with_current_line(|line, _| {
                    // pushing to the end of line unconditionally because
                    // NL is always the end of a line, and if received a CR
                    // before NL, then `with_current_span` would behave incorrect
                    if let Some(span) = line.0.last_mut() {
                        span.push(Cell::NEWLINE);
                        span.shrink();
                    }
                });
                self.buffer.set_cursor_col(0);
                self.buffer.move_cursor_down(1);
                self.buffer
                    .push_line(Line::reserve_new(self.buffer.width() as usize));
                // }
                // #[cfg(feature = "gui")] // fills span to ScreenBuffer::width()
                // {
                //     let remainder = self.buffer.width() as usize
                //         - (self.buffer.curr_line().map_or_else(|| 0, Line::num_cells));
                //     if remainder != 0 {
                //         self.buffer.with_current_span(|span, _| {
                //             let width = remainder + span.cells.len();
                //             span.fill_to_width(width);
                //         });
                //     }
                //     self.buffer
                //         .push_line(Line::reserve_new(self.buffer.width() as usize));
                //     self.buffer.set_cursor_col(0);
                // }
            }
            TAB => {
                self.buffer.with_current_span(|span, _| {
                    span.push(Cell::TAB);
                });
                self.buffer.cursor.tab();
            }
            CR => {
                self.buffer.with_current_span(|span, _| {
                    span.push(Cell::CARRIGE);
                    span.shrink();
                });
                self.buffer.set_cursor_col(0);
            }
            // Not sure that FF needs to be handled
            #[allow(clippy::match_same_arms)]
            FF => {}
            _ => {}
        }
    }

    fn handle_escape(&mut self, seq: &[u8]) {
        let Some(seq_type) = classify_escape_seq(seq) else {
            return;
        };
        match seq_type {
            EscSequenceType::Cursor(kind) => {
                process_cursor(seq, kind, self.buffer, self.stdout);
            }
            EscSequenceType::Erase(kind) => process_erase(seq, kind, self.buffer, self.stdout),
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

impl std::fmt::Display for EscSequenceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cursor(b) => todo!(),
            Self::Erase(b) => todo!(),
            Self::Graphics => todo!(),
            Self::Screen(b) => todo!(),
        }
    }
}

pub(crate) fn classify_escape_seq(seq: &[u8]) -> Option<EscSequenceType> {
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
