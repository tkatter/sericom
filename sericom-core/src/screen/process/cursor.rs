use crate::screen::{
    ScreenBuffer, TermPos,
    position::{Cursor, Position},
    process::SEP,
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

pub fn process_cursor<W: std::io::Write>(
    seq: &[u8],
    kind: u8,
    sb: &mut ScreenBuffer,
    stdout: &mut W,
) {
    let body = &seq[2..seq.len() - 1];

    // ESC[{row};{col}H
    // move cursor to line # col #
    if (kind == b'H' || kind == b'f') && !body.is_empty() {
        let mut parts = body.split(|&b| b == SEP);
        let row = parts.next().and_then(ascii_digits_to_integer);
        let col = parts.next().and_then(ascii_digits_to_integer);
        if let (Some(c), Some(r)) = (col, row) {
            sb.set_cursor_pos((c, r));
        }
    } else if kind == b'n' && body == [b'6'] {
        // request cursor pos
        // need to send to device via ESC[row;colR
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
            b's' => sb.save_cursor_pos(stdout), // can use crossterm
            b'u' => sb.restore_cursor_pos(stdout), // can use crossterm
            b'H' => sb.set_cursor_pos(Position::ORIGIN),
            _ => {}
        }
    }
}
