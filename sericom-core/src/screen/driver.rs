use crossterm::style::Attributes;

use crate::screen::ScreenBuffer;
use crate::ui::{BK, BS, CR, ESC, FF, Line, NL, TAB};
use crate::ui::{Cell, ColorState, Cursor, ParserEvent, Span, TranslatePos};
use crate::ui::{process_colors, process_cursor};

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
        for ev in events {
            match ev {
                ParserEvent::Text(bytes) => self.write_text(&bytes),
                ParserEvent::Control(ctrl) => self.handle_control(ctrl),
                ParserEvent::EscapeSequence(seq) => self.handle_escape(&seq),
            }
        }
        todo!()
    }

    fn write_text(&mut self, bytes: &[u8]) {
        let chars: Vec<char> = bytes.iter().map(|b| char::from(*b)).collect();
        let num_chars = chars.len();

        self.buffer.with_current_span(|span| {
            for ch in chars {
                span.push(Cell::new(ch));
            }
        });
        self.buffer.move_cursor_right(num_chars as u16);
    }

    fn handle_control(&mut self, ctrl: u8) {
        match ctrl {
            BS => self.buffer.move_cursor_left(1),
            CR => self.buffer.set_cursor_col(0),
            // Need to handle creating new empty buffer
            FF => todo!(),
            // FF => queue!(writer, Clear(ClearType::All)).into_diagnostic()?,
            NL => {
                let remainder =
                    self.buffer.rect.width as usize - (self.buffer.curr_line().num_cells());
                if remainder != 0 {
                    self.buffer.with_current_span(|span| {
                        span.fill_to_width(remainder);
                    });
                }
                self.buffer.move_cursor_down(1);
            }
            TAB => todo!(),
            _ => {}
        }
    }

    fn handle_escape(&mut self, seq: &[u8]) {
        let Some(seq_type) = classify_escape_seq(seq) else {
            todo!();
        };
        match seq_type {
            EscSequenceType::Cursor(kind) => {
                // TODO: Figure out span-splitting
                process_cursor(seq, kind, self.buffer);
            }
            EscSequenceType::Erase(_kind) => todo!(),
            EscSequenceType::Graphics => {
                process_colors(seq, &mut self.color_state, &mut self.attrs);
                self.buffer.with_current_span(|span| {
                    if !span.is_empty() {
                        span.shrink();
                        // self.curr_line.push(curr_span);
                        // curr_span = Span::reserve_new(
                        //     span_cap(&self.curr_line, &self.rect),
                        //     None,
                        //     None,
                        // );
                    }
                    span.set_attrs(self.attrs);
                    span.set_colors(&self.color_state);
                });
            }
            EscSequenceType::Screen(_kind) => todo!(),
        }
    }
}

impl ScreenBuffer {
    fn with_current_span<F: FnOnce(&mut Span)>(&mut self, f: F) {
        let buff_pos = self.to_buff(self.cursor);

        let span = {
            let line = self.curr_line_mut();
            let (span_idx, _col_offset) = line.span_at_col(buff_pos.x as usize);
            line.get_mut_span(span_idx).expect("verified")
        };

        f(span);
    }

    fn curr_line(&mut self) -> &Line {
        let pos_in_lines = self.to_buff(self.cursor);
        if self.lines.get(pos_in_lines.y as usize).is_some() {
            self.lines
                .get(pos_in_lines.y as usize)
                .expect("verified that line exists")
        } else {
            self.push_line(Line::reserve_new(self.rect.width as usize));
            self.lines.back().expect("is not empty")
        }
    }

    fn curr_line_mut(&mut self) -> &mut Line {
        let pos_in_lines = self.to_buff(self.cursor);
        if self.lines.get(pos_in_lines.y as usize).is_some() {
            self.lines
                .get_mut(pos_in_lines.y as usize)
                .expect("verified that line exists")
        } else {
            self.push_line(Line::reserve_new(self.rect.width as usize));
            self.lines.back_mut().expect("is not empty")
        }
    }
}

enum EscSequenceType {
    Cursor(u8),
    Erase(u8),
    Graphics,
    Screen(u8),
}

fn classify_escape_seq(seq: &[u8]) -> Option<EscSequenceType> {
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
