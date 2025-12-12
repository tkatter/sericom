use super::*;
use crate::{assert_line_eq, setup};

/// Bracket '['
const BK: u8 = b'[';
/// Backspace
const BS: u8 = 0x08;
/// Carrige return '\r'
const CR: u8 = 0x0D;
/// Escape 'ESC'
const ESC: u8 = 0x1B;
/// Newline '\n'
const NL: u8 = 0x0A;
/// Escape sequence separator ';'
const SEP: u8 = b';';
/// Tab '\t'
const TAB: u8 = 0x09;
/// Form feed
const FF: u8 = 0x0C;
/// Reset graphics mode escape sequence
const RESET: &[u8] = &[ESC, BK, b'0', b'm'];

#[test]
fn clear_line_from_cursor() {
    setup!(sb, parser, _config, stdout);
    let s = "This is the first line\nThis is the second line\nThis is the third line\n";
    let s = [
        s.as_bytes(),
        &[ESC, BK, b'1', SEP, b'1', b'0', b'H'], // move cursor to (10, 1)
        &[ESC, BK, b'K'],                        // clear line from cursor
    ]
    .concat();

    let parsed = parser.feed(&s);
    let mut driver = ScreenDriver::new(&mut sb);
    driver.process_events(parsed);

    assert_eq!(sb.lines.len(), 4);
    assert_eq!(sb.cursor, Position::<TermPos>::from((10u16, 1u16)));
    assert_line_eq!(sb, 1, "This is th");
}

#[test]
fn clear_from_cursor_to_top() {
    setup!(sb, parser, _config, stdout);
    let s = "This is the first line\n".to_owned()
        + "This is the second line\n"
        + "This is the third line\n"
        + "This is the fourth line\n"
        + "This is the fifth line\n";
    let s = [
        s.as_bytes(),
        &[ESC, BK, b'3', SEP, b'1', b'0', b'H'], // move cursor to (10, 3)
        &[ESC, BK, b'1', b'J'],                  // clear from cursor to beginning
    ]
    .concat();

    let parsed = parser.feed(&s);
    let mut driver = ScreenDriver::new(&mut sb);
    driver.process_events(parsed);

    assert_eq!(sb.lines.len(), 6);
    assert_eq!(sb.cursor, Position::<TermPos>::from((10u16, 3u16)));
    assert_line_eq!(sb, 0, "");
    assert_line_eq!(sb, 1, "");
    assert_line_eq!(sb, 2, "");
    assert_line_eq!(sb, 3, "          e fourth line");
    assert_line_eq!(sb, 4, "This is the fifth line");
}

#[test]
fn clear_and_overwrite() {
    setup!(sb, parser, _config, stdout);
    let s = "This is the first line\n".to_owned()
        + "This is the second line\n"
        + "This is the third line\n"
        + "This is the fourth line\n"
        + "This is the fifth line\n";
    let s = [
        s.as_bytes(),
        &[ESC, BK, b'3', SEP, b'1', b'0', b'H'], // move cursor to (10, 3)
        &[ESC, BK, b'1', b'J'],                  // clear from cursor to beginning
        b"\rNew words".as_slice(),
    ]
    .concat();

    let parsed = parser.feed(&s);
    let mut driver = ScreenDriver::new(&mut sb);
    driver.process_events(parsed);

    assert_eq!(sb.lines.len(), 6);
    // assert_eq!(sb.cursor, Position::<TermPos>::from((10u16, 3u16)));
    assert_line_eq!(sb, 0, "");
    assert_line_eq!(sb, 1, "");
    assert_line_eq!(sb, 2, "");
    assert_line_eq!(sb, 3, "New words e fourth line");
    assert_line_eq!(sb, 4, "This is the fifth line");
}
