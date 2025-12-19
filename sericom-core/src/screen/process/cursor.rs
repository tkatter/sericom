use crate::screen::{
    ScreenBuffer,
    position::{Cursor, Position},
    process::{SEMI, digits_to_int},
};

pub fn process_cursor(seq: &[u8], kind: u8, sb: &mut ScreenBuffer) {
    let body = &seq[2..seq.len() - 1];

    // ESC[{row};{col}H
    // move cursor to line # col #
    if (kind == b'H' || kind == b'f') && !body.is_empty() {
        let mut parts = body.split(|&b| b == SEMI);
        let row = parts.next().and_then(digits_to_int);
        let col = parts.next().and_then(digits_to_int);
        if let (Some(c), Some(r)) = (col, row) {
            sb.set_cursor_pos((c, r));
        }
    } else if kind == b'n' && body == [b'6'] {
        // request cursor pos
        // need to send to device via ESC[row;colR
        todo!();
    } else {
        let nums = digits_to_int(body);
        match kind {
            b'A' => {
                // move cursor up # lines
                if let Some(n) = nums {
                    sb.move_cursor_up(n);
                }
            }
            b'B' => {
                // move cursor down # lines
                if let Some(n) = nums {
                    sb.move_cursor_down(n);
                }
            }
            b'C' => {
                // move cursor right # cols
                if let Some(n) = nums {
                    sb.move_cursor_right(n);
                }
            }
            b'D' => {
                // move cursor left # cols
                if let Some(n) = nums {
                    sb.move_cursor_left(n);
                }
            }
            b'E' => {
                // move to beginning of next line,
                if let Some(n) = nums {
                    sb.set_cursor_col(0);
                    sb.move_cursor_down(n);
                }
            }
            b'F' => {
                // move to beginning of prev line,
                if let Some(n) = nums {
                    sb.set_cursor_col(0);
                    sb.move_cursor_up(n);
                }
            }
            b'G' => {
                // move cursor to column #
                if let Some(n) = nums {
                    sb.set_cursor_col(n);
                }
            }
            b's' => todo!(), // sb.save_cursor_pos(stdout), // can use crossterm
            b'u' => todo!(), // sb.restore_cursor_pos(stdout), // can use crossterm
            b'H' => sb.set_cursor_pos(Position::ORIGIN),
            _ => {}
        }
    }
}
