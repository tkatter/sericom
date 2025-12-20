mod colors;
mod driver;
mod parser;
mod screen;
mod xterm;

pub use colors::{ColorState, process_colors};
pub use driver::ScreenDriver;
pub use parser::{ByteParser, ParseState, ParserEvent};
pub use xterm::*;

pub const fn digits_to_int(body: &[u8]) -> Option<u16> {
    if body.is_empty() {
        return None;
    }

    let mut acc: u16 = 0;
    let mut i = 0;
    while i < body.len() {
        let b = body[i];
        if (b < b'0') || (b > b'9') {
            return None;
        }
        acc = acc * 10 + (b & 0x0F) as u16;
        i += 1;
    }
    Some(acc)
}
