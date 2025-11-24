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
            match ev {
                ParserEvent::Text(bytes) => self.write_text(&bytes),
                ParserEvent::Control(ctrl) => self.handle_control(ctrl),
                ParserEvent::EscapeSequence(seq) => self.handle_escape(&seq),
            }
        }
    }

    // TODO: HANDLE CHECKING TO SEE IF THE CURSOR IS OVER AN EXISTING
    // SPAN/LINE BEFORE WRITING - IF SO, NEED TO SPLIT APPROPRIATLY
    fn write_text(&mut self, bytes: &[u8]) {
        let chars: Vec<char> = bytes.iter().map(|b| char::from(*b)).collect();
        let num_chars = chars.len();

        self.buffer.with_current_span(|span| {
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
                let remainder =
                    self.buffer.width() as usize - (self.buffer.curr_line().num_cells());
                if remainder != 0 {
                    self.buffer.with_current_span(|span| {
                        let width = remainder + span.cells.len();
                        span.fill_to_width(width);
                    });
                }
                self.buffer
                    .push_line(Line::reserve_new(self.buffer.width() as usize));
                self.buffer.set_cursor_col(0);
                self.buffer.move_cursor_down(1);
            }
            TAB => self.buffer.with_current_span(|span| {
                span.push(Cell::TAB);
            }),
            // Need to make the events peekable to handle '\r\n' for now
            // lets just roll with ignoring it and see what happens
            #[allow(clippy::match_same_arms)]
            CR => {} // self.buffer.set_cursor_col(0),
            // Not sure that FF needs to be handled
            #[allow(clippy::match_same_arms)]
            FF => {}
            _ => {}
        }
    }

    fn handle_escape(&mut self, seq: &[u8]) {
        let Some(seq_type) = classify_escape_seq(seq) else {
            todo!();
        };
        match seq_type {
            EscSequenceType::Cursor(kind) => {
                process_cursor(seq, kind, self.buffer, self.stdout);
            }
            EscSequenceType::Erase(kind) => process_erase(seq, kind, self.buffer),
            EscSequenceType::Graphics => {
                process_colors(seq, &mut self.color_state, &mut self.attrs);
                self.buffer
                    .handle_span_colors(&self.color_state, self.attrs);
            }
            EscSequenceType::Screen(_kind) => {} // process_screen(seq, kind, self.buffer),
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

fn classify_escape_seq(seq: &[u8]) -> Option<EscSequenceType> {
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
