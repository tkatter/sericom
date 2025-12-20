mod colors;
mod components;
mod cursor;
mod escape;

pub use crate::screen::{ScreenDriver, *};
pub use crossterm::style::{Attribute, Attributes, Color};
pub use std::collections::VecDeque;

pub const TERMINAL_SIZE: (u16, u16) = (80, 24);

#[macro_export]
macro_rules! setup {
    ($sb:ident, $parser:ident) => {
        $crate::configs::init_for_tests();
        let rect = Rect::new(Position::ORIGIN, TERMINAL_SIZE.0, TERMINAL_SIZE.1);
        let mut $sb = ScreenBuffer::new(rect);
        let mut $parser = ByteParser::new();
    };
    ($sb:ident, $parser:ident, $config:ident) => {
        $crate::configs::init_for_tests();
        let rect = Rect::new(Position::ORIGIN, TERMINAL_SIZE.0, TERMINAL_SIZE.1);
        let mut $sb = ScreenBuffer::new(rect);
        let mut $parser = ByteParser::new();
        let $config = $crate::configs::get_config().unwrap();
    };
}

/// Assert that a line's text contents equal `expected`
/// Usage:
///   `assert_line_eq!(sb, 1, "Hello, world!");`
#[macro_export]
macro_rules! assert_line_eq {
    ($sb:expr, $line_idx:expr, $expected:expr) => {{
        let sb = &$sb; // ScreenBuffer
        let line_idx = $line_idx; // which line
        let expected: &str = $expected; // expected text

        let line = sb.lines.get(line_idx).expect(&format!(
            "No line at index {line_idx}, only {} lines",
            sb.lines.len()
        ));
        let actual: String = line.iter().flatten().collect();

        if actual.trim_end() != expected {
            panic!(
                "Line {line_idx} mismatch!\n  Expected: {:?}\n  Actual:   {:?}\n\nFull buffer:\n{}",
                expected,
                actual,
                $crate::screen::tests::debug_dump(&sb.lines),
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
            let actual_text: String = span.iter().collect();
            if !actual_text.starts_with(expected_text) {
                panic!(
                    "Span {span_idx} text mismatch!\n  Expected: {:?}\n  Actual:   {:?}\n\nFull buffer:\n{}",
                    expected_text,
                    actual_text,
                $crate::screen::tests::debug_dump(&sb.lines),
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
                $crate::screen::tests::debug_dump(&sb.lines),
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
                $crate::screen::tests::debug_dump(&sb.lines),
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
                $crate::screen::tests::debug_dump(&sb.lines),
            );
        )?
    }};
}

pub fn debug_dump(lines: &VecDeque<Line>) -> String {
    let mut out = String::new();
    for (i, line) in lines.iter().enumerate() {
        use std::fmt::Write;

        writeln!(&mut out, "Line {i}:").unwrap();

        for (j, span) in line.iter().enumerate() {
            let text: String = span.iter().map(|c| c.character as char).collect();
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
