use crate::setup;
use std::fmt::Write;
use std::io;

use super::*;
use crossterm::execute;
use crossterm::style::Colors;
use crossterm::style::SetAttribute;
use crossterm::style::SetColors;
use crossterm::style::SetForegroundColor;

// Stole this from crossterm's test - thanks!
#[derive(Default, Debug, Clone)]
struct FakeWrite {
    buffer: String,
    flushed: bool,
}

impl io::Write for FakeWrite {
    fn write(&mut self, content: &[u8]) -> io::Result<usize> {
        let content =
            str::from_utf8(content).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        self.buffer.push_str(content);
        self.flushed = false;
        Ok(content.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.flushed = true;
        Ok(())
    }
}

#[test]
fn line_as_command() {
    initialize_config(CONFIG_OVERRIDE).ok();
    let mut writer = FakeWrite {
        buffer: String::new(),
        flushed: false,
    };
    let mut colors = ColorState::default();
    colors.set_fg(Color::Red);
    let mut attrs = Attributes::default();
    attrs.set(Attribute::Bold);

    let span1: Span = "first span".chars().collect();
    let mut span2: Span = "second span".chars().collect();
    span2.set_colors(&colors);
    span2.set_attrs(attrs);
    let mut span3: Span = "third span\n".chars().collect();
    let line = Line(vec![span1, span2, span3]);

    execute!(writer, line);

    let mut cmp = String::new();
    write!(cmp, "{}", SetColors(Colors::new(Color::Cyan, Color::Reset)));
    write!(cmp, "first span");
    write!(cmp, "{}", SetColors(Colors::new(Color::Red, Color::Reset)));
    write!(cmp, "{}", SetAttribute(Attribute::Bold));
    write!(cmp, "second span");
    write!(cmp, "{}", SetColors(Colors::new(Color::Cyan, Color::Reset)));
    writeln!(cmp, "third span");

    assert_eq!(writer.buffer, cmp);
}

#[test]
fn overwriting_span() {
    let mut span = vec![Cell::EMPTY; 10];
    let chars = vec!['a'; 10];

    for (cell, ch) in span.iter_mut().skip(5).zip(chars) {
        cell.character = ch;
    }

    assert_eq!(span, [[Cell::EMPTY; 5], [Cell::new('a'); 5]].concat());
}

#[test]
fn span_newline() {
    initialize_config(CONFIG_OVERRIDE);

    let mut span: Span = "my test span       ".chars().collect();
    let res = span.last_filled_idx();
    assert_eq!(12, res);
    {
        let c = span
            .cells
            .get_mut(res)
            .expect("span len is greater than last filled cell");
        c.character = '\n';
    }

    let res = span.last_filled_idx();
    assert_eq!(13, res);

    span.shrink();

    assert_eq!(span.cells.capacity(), 13);
    assert_eq!(span.cells.len(), 13);
}

#[test]
fn line_last_filled() {
    initialize_config(CONFIG_OVERRIDE);

    let mut span1: Span = "first span".chars().collect();
    let mut span2: Span = "second span       ".chars().collect();
    let line = Line(vec![span1, span2]);

    assert_eq!(line.last_filled_idx(), 20);
    assert_eq!(line.iter().flatten().nth(20), Some(&Cell::new('n')));
}

#[test]
fn line_num_filled() {
    initialize_config(CONFIG_OVERRIDE);

    let mut span1: Span = "first span".chars().collect();
    let mut span2: Span = "second span       ".chars().collect();
    let line = Line(vec![span1, span2]);

    assert_eq!(line.filled_cells(), 21);

    let mut span: Span = "                  ".chars().collect();
    let line = Line(vec![span]);

    assert_eq!(line.filled_cells(), 0);
}
