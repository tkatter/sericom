use crossterm::style::Attributes;

use crate::{
    screen_buffer::ScreenBuffer,
    ui::{
        BK, BS, CR, Cell, Cursor, ESC, FF, Line, NL, ParserEvent, Rect, Span, TAB,
        ascii::cursor::process_cursor, line::ColorState, process_colors,
    },
};

impl ScreenBuffer {
    pub(crate) fn process_events(&mut self, events: Vec<ParserEvent>) {
        let mut color_state = ColorState::default();
        let mut attrs = Attributes::default();

        let mut curr_line = Line::reserve_new(usize::from(self.rect.width));

        let span_cap = |line: &Line, rect: &Rect| -> usize {
            if !line.is_empty() {
                return usize::from(rect.width) - line.num_cells();
            }
            usize::from(rect.width)
        };

        let mut curr_span = Span::reserve_new(span_cap(&curr_line, &self.rect), None, None);

        for ev in events {
            match &ev {
                ParserEvent::Text(t) => {
                    for c in t {
                        let total_cells = curr_line.num_cells() + curr_span.len();

                        // Don't want an else branch because don't want line wrap
                        if total_cells < usize::from(self.rect.width) {
                            curr_span.push(Cell::new(char::from(*c)));
                            self.move_cursor_right(1);
                        }
                    }
                }
                ParserEvent::Control(b) => {
                    match *b {
                        BS => self.move_cursor_left(1),
                        CR => self.set_cursor_col(0),
                        // Need to handle creating new empty buffer
                        FF => todo!(),
                        // FF => queue!(writer, Clear(ClearType::All)).into_diagnostic()?,
                        NL => {
                            let remainder = usize::from(self.rect.width)
                                - (curr_span.len() + curr_line.num_cells());
                            if remainder != 0 {
                                curr_span.fill_to_width(span_cap(&curr_line, &self.rect));
                                curr_line.push(curr_span);
                                curr_span = Span::reserve_new(
                                    span_cap(&curr_line, &self.rect),
                                    Some(color_state.get_colors()),
                                    Some(attrs),
                                );
                            }
                            self.push_line(curr_line);
                            curr_line = Line::reserve_new(usize::from(self.rect.width));
                        }
                        TAB => todo!(),
                        _ => {}
                    }
                }
                ParserEvent::EscapeSequence(seq) => {
                    let Some(seq_type) = classify_escape_seq(seq) else {
                        todo!();
                    };
                    match seq_type {
                        EscSequenceType::Cursor(kind) => {
                            process_cursor(seq, kind, self);
                        }
                        EscSequenceType::Erase(_kind) => todo!(),
                        EscSequenceType::Graphics => {
                            if !curr_span.is_empty() {
                                curr_span.shrink();
                                curr_line.push(curr_span);
                                curr_span =
                                    Span::reserve_new(span_cap(&curr_line, &self.rect), None, None);
                            }
                            process_colors(seq, &mut color_state, &mut attrs);
                            curr_span.set_attrs(attrs);
                            curr_span.set_colors(&color_state);
                        }
                        EscSequenceType::Screen(_kind) => todo!(),
                    }
                }
            }
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
