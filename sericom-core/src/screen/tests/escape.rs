use super::*;
use crate::{assert_line_eq, assert_span_eq, csi, setup};

#[test]
fn single_plain_line() {
    setup!(sb, parser, config);
    let parsed = parser.feed(b"Hello, world!\n");
    let mut driver = ScreenDriver::new(&mut sb);
    driver.process_events(parsed);
    let fg = Color::from(&config.appearance.fg);
    let bg = Color::from(&config.appearance.bg);
    drop(config);

    assert_eq!(sb.cursor, Position::<TermPos>::from((1_u16, 2_u16)));
    // Expect one line with one span, fg=default, text padded
    assert_line_eq!(sb, 0, "Hello, world!");
    assert_span_eq!(sb, 0, 0, fg => fg, bg => bg);
}

#[test]
fn two_lines_plain_text() {
    setup!(sb, parser, config);
    let parsed = parser.feed(b"Hello\r\nWorld\r\n");
    let mut driver = ScreenDriver::new(&mut sb);
    driver.process_events(parsed);
    let fg = Color::from(&config.appearance.fg);
    let bg = Color::from(&config.appearance.bg);
    drop(config);

    // Expected: two lines, one with "Hello" padded, one with "World" padded
    assert_eq!(sb.lines.len(), 3);
    assert_eq!(sb.cursor, Position::<TermPos>::from((1_u16, 3_u16)));
    assert_line_eq!(sb, 0, "Hello");
    assert_line_eq!(sb, 1, "World");
    assert_span_eq!(sb, 0, 0, fg => fg, bg => bg);
    assert_span_eq!(sb, 1, 0, fg => fg, bg => bg);
}

#[test]
fn three_color_spans() {
    setup!(sb, parser, config);
    let s = format!(
        "{}Red{}Green{}Blue\n",
        csi!("31m"),
        csi!("32m"),
        csi!("34m")
    );
    let parsed = parser.feed(s.as_bytes());
    let mut driver = ScreenDriver::new(&mut sb);
    driver.process_events(parsed);
    let bg = Color::from(&config.appearance.bg);
    drop(config);

    let line = sb.lines.front().unwrap();
    assert_eq!(sb.lines.len(), 2);
    assert_eq!(sb.cursor, Position::<TermPos>::from((1_u16, 2_u16)));
    assert_eq!(line.len(), 3); // three spans
    assert_span_eq!(sb, 0, 0, expected => "Red", fg => Color::DarkRed, bg => bg);
    assert_span_eq!(sb, 0, 1, expected => "Green", fg => Color::DarkGreen, bg => bg);
    assert_span_eq!(sb, 0, 2, expected => "Blue", fg => Color::DarkBlue, bg => bg);
}

#[test]
fn no_newline_incomplete_line() {
    setup!(sb, parser, config);
    let parsed = parser.feed(b"Hello");
    let mut driver = ScreenDriver::new(&mut sb);
    driver.process_events(parsed);
    let fg = Color::from(&config.appearance.fg);
    let bg = Color::from(&config.appearance.bg);
    drop(config);

    // Should still only contain the initial empty line
    assert_eq!(sb.lines.len(), 1);
    assert_eq!(sb.cursor, Position::<TermPos>::from((6_u16, 1_u16)));
    assert_span_eq!(sb, 0, 0, expected => "Hello", fg => fg, bg => bg);
}

#[test]
fn mixed_plain_and_color() {
    setup!(sb, parser, config);
    let s = format!("Normal {}Red\n", csi!("31m"));
    let parsed = parser.feed(s.as_bytes());
    let mut driver = ScreenDriver::new(&mut sb);
    driver.process_events(parsed);
    let fg = Color::from(&config.appearance.fg);
    let bg = Color::from(&config.appearance.bg);
    drop(config);

    // Expect two spans: "Normal " default, "Red" DarkRed
    let line = sb.lines.front().unwrap();
    assert_eq!(sb.lines.len(), 2);
    assert_eq!(line.len(), 2);
    assert_eq!(sb.cursor, Position::<TermPos>::from((1_u16, 2_u16)));
    assert_span_eq!(sb, 0, 0, expected => "Normal ", fg => fg, bg => bg);
    assert_span_eq!(sb, 0, 1, expected => "Red", fg => Color::DarkRed, bg => bg);
}

#[test]
fn bold_italic_span() {
    setup!(sb, parser);

    let parsed = parser.feed(b"\x1b[1;3mHello\n"); // bold + italic
    let mut driver = ScreenDriver::new(&mut sb);
    driver.process_events(parsed);

    let span_attrs = Attributes::from(Attribute::Bold) | Attributes::from(Attribute::Italic);
    assert_span_eq!(sb, 0, 0, attrs => span_attrs);
}

#[test_log::test]
#[allow(clippy::cognitive_complexity)]
fn multiline_multicolor() {
    setup!(sb, parser, config);
    let fg = Color::from(&config.appearance.fg);
    let bg = Color::from(&config.appearance.bg);
    drop(config);

    let input = concat!(
        // Line 1: basic color DarkRed -> text
        "\x1b[31mRed ",
        // switch to Cyan
        "\x1b[96mCyan ",
        // switch to bold, underline, DarkBlue, then some text
        "\x1b[1;4;34mBoldUnderBlue\n",
        // Line 2: 256-color palette FG (202 = orange) and BG (27 = blueish)
        "\x1b[38;5;202;48;5;27mOrangeOnBlue ",
        // reset, then normal text
        "\x1b[0mResetHere\n",
        // Line 3: truecolor foreground text
        "\x1b[38;2;128;200;64mTrueColorGreenish ",
        // add truecolor background text with italic attr
        "\x1b[3;48;2;200;64;128mBgPinkItalic\n",
    )
    .as_bytes();

    let parsed = parser.feed(input);
    let mut driver = ScreenDriver::new(&mut sb);
    driver.process_events(parsed);

    // Buffer should have initial empty + 3 lines
    assert_eq!(sb.lines.len(), 4);

    // Line 1
    assert_line_eq!(sb, 0, "Red Cyan BoldUnderBlue");
    assert_span_eq!(sb, 0, 0, expected => "Red", fg => Color::DarkRed, bg => bg);
    assert_span_eq!(sb, 0, 1, expected => "Cyan", fg => Color::Cyan, bg => bg);
    let span_attrs = Attributes::from(Attribute::Bold) | Attributes::from(Attribute::Underlined);
    assert_span_eq!(sb, 0, 2, expected => "BoldUnderBlue", fg => Color::DarkBlue, bg => bg, attrs => span_attrs);

    // Line 2
    assert_line_eq!(sb, 1, "OrangeOnBlue ResetHere");
    assert_span_eq!(sb, 1, 0, expected => "OrangeOnBlue", fg => Color::AnsiValue(202), bg => Color::AnsiValue(27));
    assert_span_eq!(sb, 1, 1, expected => "ResetHere", fg => fg, bg => bg, attrs => Attributes::default());

    // Line 3
    assert_line_eq!(sb, 2, "TrueColorGreenish BgPinkItalic");
    assert_span_eq!(sb, 2, 0, expected => "TrueColorGreenish", fg => Color::Rgb { r:128, g:200, b:64 }, bg => bg);
    let italic = Attributes::from(Attribute::Italic);
    assert_span_eq!(sb, 2, 1, expected => "BgPinkItalic", fg => Color::Rgb { r:128, g:200, b:64 }, bg => Color::Rgb { r:200, g:64, b:128 }, attrs => italic);
}
