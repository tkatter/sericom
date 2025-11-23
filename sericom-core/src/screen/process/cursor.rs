use crate::{
    screen::ScreenBuffer,
    screen::process::SEP,
    ui::{Cursor, Position},
};

fn ascii_digits_to_integer(body: &[u8]) -> Option<u16> {
    if body.is_empty() || !body.iter().all(u8::is_ascii_digit) {
        return None;
    }
    Some(
        body.iter()
            .fold(0u16, |acc, b| acc * 10 + u16::from(b & 0x0F)),
    )
}

pub fn process_cursor(seq: &[u8], kind: u8, sb: &mut ScreenBuffer) {
    let body = &seq[2..seq.len() - 1];

    // ESC[{row};{col}H
    // move cursor to line # col #
    if (kind == b'H' && !body.is_empty()) || kind == b'f' {
        let mut parts = body.split(|&b| b == SEP);
        let row = parts.next().and_then(ascii_digits_to_integer);
        let col = parts.next().and_then(ascii_digits_to_integer);
        if let (Some(r), Some(c)) = (row, col) {
            sb.set_cursor_pos((r, c));
        }
    } else if kind == b'n' && body == [b'6'] {
        // request cursor pos
        todo!();
    } else {
        let nums = ascii_digits_to_integer(body);
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
            b's' => sb.save_cursor_pos(),
            b'u' => sb.restore_cursor_pos(),
            b'H' => sb.set_cursor_pos(Position::ORIGIN),
            _ => {}
        }
    }
}
