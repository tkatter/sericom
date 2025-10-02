use std::collections::VecDeque;

use crossterm::style::{Attribute, Attributes, Color};

use super::*;
use crate::{
    configs::{ConfigOverride, initialize_config},
    screen_buffer::*,
    ui::{Line, Position, Rect},
};
const CONFIG_OVERRIDE: ConfigOverride = ConfigOverride {
    color: None,
    out_dir: None,
    exit_script: None,
};
const TERMINAL_SIZE: (u16, u16) = (80, 24);

macro_rules! setup {
    ($sb:ident, $parser:ident) => {
        initialize_config(CONFIG_OVERRIDE).ok();
        let rect = Rect::new(Position::ORIGIN, TERMINAL_SIZE.0, TERMINAL_SIZE.1);
        let mut $sb = ScreenBuffer::new(rect);
        let mut $parser = ByteParser::new();
    };
    ($sb:ident, $parser:ident, $config:ident) => {
        initialize_config(CONFIG_OVERRIDE).ok();
        let rect = Rect::new(Position::ORIGIN, TERMINAL_SIZE.0, TERMINAL_SIZE.1);
        let mut $sb = ScreenBuffer::new(rect);
        let mut $parser = ByteParser::new();
        let $config = $crate::configs::get_config();
    };
}

/// Assert that a line's text contents equal `expected`
/// Usage:
///   `assert_line_eq!(sb, 1, "Hello, world!");`
macro_rules! assert_line_eq {
    ($sb:expr, $line_idx:expr, $expected:expr) => {{
        let sb = &$sb; // ScreenBuffer
        let line_idx = $line_idx; // which line
        let expected: &str = $expected; // expected text

        let line = sb.lines.get(line_idx).expect(&format!(
            "No line at index {line_idx}, only {} lines",
            sb.lines.len()
        ));
        let actual: String = line
            .iter()
            .flat_map(|span| span.iter().map(|c| c.character))
            .collect();

        if actual.trim_end() != expected {
            panic!(
                "Line {line_idx} mismatch!\n  Expected: {:?}\n  Actual:   {:?}\n\nFull buffer:\n{}",
                expected,
                actual,
                $crate::ui::test::debug_dump(&sb.lines),
            );
        }
    }};
}

/// Assert that a specific span on a line matches given text and style.
/// Usage:
///   `assert_span_eq!(sb, 1, 0, "Red", fg=Color::DarkRed, bg=Color::Reset);`
#[macro_export]
macro_rules! assert_span_eq {
    (
        $sb:expr,        // ScreenBuffer
        $line_idx:expr,  // which line
        $span_idx:expr  // which span in that line
        $(, expected => $expected_text:expr)?
        $(, fg => $fg_color:expr)?
        $(, bg => $bg_color:expr)?
        $(, attrs => $attrs:expr)?
    ) => {{
        let sb = &$sb;
        let line_idx = $line_idx;
        let span_idx = $span_idx;

        let line = sb
            .lines
            .get(line_idx)
            .expect(&format!("No line at index {line_idx}, only {} lines", sb.lines.len()));
        let span = line
            .iter()
            .nth(span_idx)
            .expect(&format!("No span at index {span_idx} in line {}", line_idx));

        // Collect text from cells
        $(
            let expected_text: &str = $expected_text;
            let actual_text: String = span.iter().map(|c| c.character).collect();
            if !actual_text.starts_with(expected_text) {
                panic!(
                    "Span {span_idx} text mismatch!\n  Expected: {:?}\n  Actual:   {:?}\n\nFull buffer:\n{}",
                    expected_text,
                    actual_text,
                    $crate::ui::test::debug_dump(&sb.lines),
                );
            }
        )?

        // Check optional style args
        $(
            assert_eq!(
                span.colors.foreground,
                Some($fg_color),
                "Span {} fg color mismatch (expected {:?}, got {:?})\n{}",
                span_idx,
                $fg_color,
                span.colors.foreground,
                $crate::ui::test::debug_dump(&sb.lines),
            );
        )?
        $(
            assert_eq!(
                span.colors.background,
                Some($bg_color),
                "Span {} bg color mismatch (expected {:?}, got {:?})\n{}",
                span_idx,
                $bg_color,
                span.colors.background,
                $crate::ui::test::debug_dump(&sb.lines),
            );
        )?
        $(
            assert_eq!(
                span.attrs,
                $attrs,
                "Span {} attrs mismatch (expected {:?}, got {:?})\n{}",
                span_idx,
                $attrs,
                span.attrs,
                $crate::ui::test::debug_dump(&sb.lines),
            );
        )?
    }};
}

fn debug_dump(lines: &VecDeque<Line>) -> String {
    let mut out = String::new();
    for (i, line) in lines.iter().enumerate() {
        use std::fmt::Write;

        writeln!(&mut out, "Line {i}:").unwrap();

        for (j, span) in line.iter().enumerate() {
            let text: String = span.iter().map(|c| c.character).collect();
            writeln!(
                &mut out,
                "  Span {}: \"{}\" (len = {}, attrs = {:?}, colors = {:?})",
                j,
                text,
                span.len(),
                span.attrs,
                span.colors
            )
            .unwrap();
        }
    }
    out
}

#[test]
fn test_single_plain_line() {
    setup!(sb, parser, config);
    let parsed = parser.feed(b"Hello, world!\n");
    sb.process_events(parsed);
    let fg = Color::from(&config.appearance.fg);
    let bg = Color::from(&config.appearance.bg);

    // Expect one line with one span, fg=default, text padded
    assert_line_eq!(sb, 1, "Hello, world!");
    assert_span_eq!(sb, 1, 0, fg => fg, bg => bg);
}

#[test]
fn test_two_lines_plain_text() {
    setup!(sb, parser, config);
    let parsed = parser.feed(b"Hello\nWorld\n");
    sb.process_events(parsed);
    let fg = Color::from(&config.appearance.fg);
    let bg = Color::from(&config.appearance.bg);

    // Expected: two lines, one with "Hello" padded, one with "World" padded
    assert_eq!(sb.lines.len(), 3); // initial empty line + 2
    assert_line_eq!(sb, 1, "Hello");
    assert_line_eq!(sb, 2, "World");
    assert_span_eq!(sb, 1, 0, fg => fg, bg => bg);
    assert_span_eq!(sb, 2, 0, fg => fg, bg => bg);
}

#[test]
fn test_three_color_spans() {
    setup!(sb, parser, config);
    let parsed = parser.feed(b"\x1b[31mRed\x1b[32mGreen\x1b[34mBlue\n");
    sb.process_events(parsed);
    let bg = Color::from(&config.appearance.bg);

    assert_eq!(sb.lines.len(), 2); // initial empty + 1 line
    let line = sb.lines.get(1).unwrap();
    assert_eq!(line.len(), 3); // three spans
    assert_span_eq!(sb, 1, 0, expected => "Red", fg => Color::DarkRed, bg => bg);
    assert_span_eq!(sb, 1, 1, expected => "Green", fg => Color::DarkGreen, bg => bg);
    assert_span_eq!(sb, 1, 2, expected => "Blue", fg => Color::DarkBlue, bg => bg);
}

#[test]
fn test_no_newline_incomplete_line() {
    setup!(sb, parser, config);
    let parsed = parser.feed(b"Hello");
    sb.process_events(parsed);
    let fg = Color::from(&config.appearance.fg);
    let bg = Color::from(&config.appearance.bg);

    // Should still only contain the initial empty line
    assert_eq!(sb.lines.len(), 1);
    assert_span_eq!(sb, 0, 0, expected => "", fg => fg, bg => bg);
}

#[test]
fn test_mixed_plain_and_color() {
    setup!(sb, parser, config);
    let parsed = parser.feed(b"Normal \x1b[31mRed\n");
    sb.process_events(parsed);
    let fg = Color::from(&config.appearance.fg);
    let bg = Color::from(&config.appearance.bg);

    // Expect two spans: "Normal " default, "Red" DarkRed
    let line = sb.lines.get(1).unwrap();
    assert_eq!(line.len(), 2);
    assert_span_eq!(sb, 1, 0, expected => "Normal ", fg => fg, bg => bg);
    assert_span_eq!(sb, 1, 1, expected => "Red", fg => Color::DarkRed, bg => bg);
}

#[test]
fn test_bold_italic_span() {
    setup!(sb, parser);

    let parsed = parser.feed(b"\x1b[1;3mHello\n"); // bold + italic
    sb.process_events(parsed);

    let span_attrs = Attributes::from(Attribute::Bold) | Attributes::from(Attribute::Italic);
    assert_span_eq!(sb, 1, 0, attrs => span_attrs);
}

#[test]
#[allow(clippy::cognitive_complexity)]
fn test_multiline_multicolor() {
    setup!(sb, parser, config);
    let fg = Color::from(&config.appearance.fg);
    let bg = Color::from(&config.appearance.bg);

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
    sb.process_events(parsed);

    // Buffer should have initial empty + 3 lines
    assert_eq!(sb.lines.len(), 4);

    // Line 1
    assert_line_eq!(sb, 1, "Red Cyan BoldUnderBlue");
    assert_span_eq!(sb, 1, 0, expected => "Red", fg => Color::DarkRed, bg => bg);
    assert_span_eq!(sb, 1, 1, expected => "Cyan", fg => Color::Cyan, bg => bg);
    let span_attrs = Attributes::from(Attribute::Bold) | Attributes::from(Attribute::Underlined);
    assert_span_eq!(sb, 1, 2, expected => "BoldUnderBlue", fg => Color::DarkBlue, bg => bg, attrs => span_attrs);

    // Line 2
    assert_line_eq!(sb, 2, "OrangeOnBlue ResetHere");
    assert_span_eq!(sb, 2, 0, expected => "OrangeOnBlue", fg => Color::AnsiValue(202), bg => Color::AnsiValue(27));
    assert_span_eq!(sb, 2, 1, expected => "ResetHere", fg => fg, bg => bg, attrs => Attributes::default());

    // Line 3
    assert_line_eq!(sb, 3, "TrueColorGreenish BgPinkItalic");
    assert_span_eq!(sb, 3, 0, expected => "TrueColorGreenish", fg => Color::Rgb { r:128, g:200, b:64 }, bg => bg);
    let italic = Attributes::from(Attribute::Italic);
    assert_span_eq!(sb, 3, 1, expected => "BgPinkItalic", fg => Color::Rgb { r:128, g:200, b:64 }, bg => Color::Rgb { r:200, g:64, b:128 }, attrs => italic);
}
