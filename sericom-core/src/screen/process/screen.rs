use std::ops::Range;

use crate::screen::{Line, Position, ScreenBuffer, TermPos, TranslatePos};

pub fn process_erase(seq: &[u8], kind: u8, sb: &mut ScreenBuffer) {
    let body = &seq[2..seq.len() - 1];

    match (kind, body) {
        // erase from cursor until end of screen
        (b'J', [] | [b'0']) => {
            let buff_range = Range {
                start: (sb.to_buff(sb.cursor).y + 1) as usize,
                end: (sb.buff_rect().bottom() + 1) as usize,
            };

            sb.clear_line_from_cursor();
            sb.clear_lines(buff_range);
        }
        // erase from cursor to beginning of screen
        (b'J', [b'1']) => {
            let buff_range = Range {
                start: sb.view_start,
                end: sb.to_buff(sb.cursor).y as usize,
            };

            sb.clear_lines(buff_range);
            sb.clear_line_to_cursor();
        }
        // erase entire screen - move cursor to ORIGIN
        // erase saved lines - same as erase entire screen
        // to preserve user scrollback history
        (b'J', [b'2' | b'3']) => {
            sb.lines.push_back(Line::new_empty(usize::from(sb.width())));
            sb.view_start = sb.lines.len().saturating_sub(1);
            sb.cursor = Position::<TermPos>::ORIGIN;
        }
        // erase from cursor until end of line
        (b'K', [] | [b'0']) => sb.clear_line_from_cursor(),
        // erase start of line to the cursor
        (b'K', [b'1']) => sb.clear_line_to_cursor(),
        // erase the entire line
        (b'K', [b'2']) => {
            sb.with_current_line(|line, _| {
                line.iter_mut().flatten().for_each(|cell| {
                    cell.character = ' ';
                });
            });
        }
        _ => {}
    }
}

impl ScreenBuffer {
    pub(crate) fn clear_line_to_cursor(&mut self) {
        self.with_current_line(|line, cursor| {
            for (idx, cell) in line.iter_mut().flatten().enumerate() {
                if idx < usize::from(cursor.x) {
                    cell.character = ' ';
                }
            }
        });
    }

    pub(crate) fn clear_line_from_cursor(&mut self) {
        self.with_current_line(|line, cursor| {
            line.iter_mut()
                .flatten()
                .skip(usize::from(cursor.x))
                .for_each(|cell| {
                    cell.character = ' ';
                });
        });
    }

    pub(crate) fn clear_lines(&mut self, range: Range<usize>) {
        for line in self.lines.range_mut(range) {
            line.iter_mut().flatten().for_each(|cell| {
                cell.character = ' ';
            });
        }
    }
}

pub fn _process_screen(seq: &[u8], _kind: u8, _sb: &mut ScreenBuffer) {
    let _body = &seq[2..seq.len() - 1];
    // ESC[={value}h Changes the screen width or type to the mode specified by value
    todo!()
}
