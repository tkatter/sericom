use super::*;
use crate::assert_line_eq;
use crate::assert_span_eq;
use crate::csi;
use crate::setup;

const WORDS: [&str; 4] = [
    "This is one line of words",
    "This is a second line of words",
    "This is a third line of words",
    "This is a fourth line of words",
];

#[test]
fn tabs() {
    setup!(sb, parser);
    {
        let s = format!(
            "This is the first line\nThis is the second line\nThis is the third line\n{}Tabbed?",
            csi!("I")
        );
        let parsed = parser.feed(s.as_bytes());
        let mut driver = ScreenDriver::new(&mut sb);
        driver.process_events(parsed);

        assert_eq!(sb.lines.len(), 4);
        assert_eq!(sb.cursor, Position::<TermPos>::from((16u16, 4u16)));
        assert_line_eq!(sb, 3, "        Tabbed?");
    }
    {
        let s = format!(
            "This is the first line\nThis is the second line\nThis is the third line\n{}Tabbed?",
            csi!("3I")
        );
        let parsed = parser.feed(s.as_bytes());
        let mut driver = ScreenDriver::new(&mut sb);
        driver.process_events(parsed);
        assert_eq!(sb.lines.len(), 7);
        assert_eq!(sb.cursor, Position::<TermPos>::from((32u16, 7u16)));
        assert_line_eq!(sb, 6, "                        Tabbed?");
    }
    {
        let s = format!("{}Reverse!", csi!("2Z"));
        let parsed = parser.feed(s.as_bytes());
        let mut driver = ScreenDriver::new(&mut sb);
        driver.process_events(parsed);
        assert_eq!(sb.lines.len(), 7);
        assert_eq!(sb.cursor, Position::<TermPos>::from((24u16, 7u16)));
        assert_line_eq!(sb, 6, "               Reverse! Tabbed?");
    }
}

#[test]
fn cursor_nextline() {
    setup!(sb, parser);
    let mut driver = ScreenDriver::new(&mut sb);
    let seq = format!("{}{}{}", WORDS[0], csi!("E"), WORDS[1]);
    let parsed = parser.feed(seq.as_bytes());
    driver.process_events(parsed);

    assert_line_eq!(sb, 0, "This is one line of words");
    assert_line_eq!(sb, 1, "This is a second line of words");
    assert_eq!(sb.cursor.x, 31);
    assert_eq!(sb.cursor.y, 2);
    let line = sb.curr_line().unwrap();
    assert_eq!(line.last_filled_idx(), 29);
    assert_eq!(sb.lines.len(), 2);
}

#[test]
fn cursor_up_down() {
    setup!(sb, parser);
    let mut driver = ScreenDriver::new(&mut sb);
    let seq = format!(
        "{}\n{}\n{}\n{}This should overwrite the first line{}NewText?\nNow another line{}Overwriting again{}Hello there!{}Above you!",
        WORDS[0],
        WORDS[1],
        WORDS[2],
        csi!("6A"),
        csi!("10B"),
        csi!("5A"),
        csi!("2E"),
        csi!("F"),
    );
    let parsed = parser.feed(seq.as_bytes());
    driver.process_events(parsed);

    assert_line_eq!(sb, 0, "This should overwrite the first line");
    assert_line_eq!(sb, 10, "                                    NewText?");
    assert_line_eq!(sb, 11, "Now another line");
    assert_line_eq!(sb, 6, "                Overwriting again");
    assert_line_eq!(sb, 8, "Hello there!");
    assert_line_eq!(sb, 7, "Above you!");
    assert_eq!(sb.cursor.y, 8);
    assert_eq!(sb.lines.len(), 12);
}

#[test]
fn clear_line_from_cursor() {
    setup!(sb, parser);
    let s = format!(
        "This is the first line\nThis is the second line\nThis is the third line\n{}{}",
        csi!("1;10H"), // move cursor to (10, 1)
        csi!("K")      // clear line from cursor
    );

    let parsed = parser.feed(s.as_bytes());
    let mut driver = ScreenDriver::new(&mut sb);
    driver.process_events(parsed);

    assert_eq!(sb.lines.len(), 4);
    assert_eq!(sb.cursor, Position::<TermPos>::from((10u16, 1u16)));
    assert_line_eq!(sb, 0, "This is t");
}

#[test]
fn clear_from_cursor_to_top() {
    setup!(sb, parser);
    let s = "This is the first line\n".to_owned()
        + "This is the second line\n"
        + "This is the third line\n"
        + "This is the fourth line\n"
        + "This is the fifth line\n";
    let s = format!("{s}{}{}", csi!("3;10H"), csi!("1J"));
    // move cursor to (10, 3)
    // clear from cursor to beginning

    let parsed = parser.feed(s.as_bytes());
    let mut driver = ScreenDriver::new(&mut sb);
    driver.process_events(parsed);

    assert_eq!(sb.lines.len(), 6);
    assert_eq!(sb.cursor, Position::<TermPos>::from((10u16, 3u16)));
    assert_line_eq!(sb, 0, "");
    assert_line_eq!(sb, 1, "");
    assert_line_eq!(sb, 2, "         he third line");
    assert_line_eq!(sb, 3, "This is the fourth line");
    assert_line_eq!(sb, 4, "This is the fifth line");
}

#[test]
fn clear_and_overwrite() {
    setup!(sb, parser);
    let s = "This is the first line\n".to_owned()
        + "This is the second line\n"
        + "This is the third line\n"
        + "This is the fourth line\n"
        + "This is the fifth line\n";
    let s = format!("{s}{}{}\rNew words", csi!("3;10H"), csi!("1J"));
    // move cursor to (10, 3)
    // clear from cursor to beginning

    let parsed = parser.feed(s.as_bytes());
    let mut driver = ScreenDriver::new(&mut sb);
    driver.process_events(parsed);

    assert_eq!(sb.lines.len(), 6);
    assert_eq!(sb.cursor, Position::<TermPos>::from((10u16, 3u16)));
    assert_line_eq!(sb, 0, "");
    assert_line_eq!(sb, 1, "");
    assert_line_eq!(sb, 2, "New wordshe third line");
    assert_line_eq!(sb, 3, "This is the fourth line");
    assert_line_eq!(sb, 4, "This is the fifth line");
}

#[test]
fn delete_chars_triple_span() {
    use crossterm::style::Color;

    setup!(sb, parser, config);
    let fg = Color::from(&config.appearance.fg);
    drop(config);
    let mut driver = ScreenDriver::new(&mut sb);
    let seq = format!(
        "{}\n{}\nThis is span 1{}This is span 2{}This is span 3{}\n{}{}{}",
        WORDS[0],
        WORDS[1],
        csi!("32m"),
        csi!("34m"),
        csi!("0m"),
        csi!("F"),
        csi!("7C"),
        csi!("28P")
    );
    let parsed = parser.feed(seq.as_bytes());
    driver.process_events(parsed);

    assert_eq!(sb.cursor.x, 8);
    assert_eq!(sb.cursor.y, 3);
    assert_line_eq!(sb, 2, "This is span 3");
    assert_span_eq!(sb, 2, 0, expected => "This is", fg => fg);
    assert_span_eq!(sb, 2, 1, expected => " span 3", fg => Color::DarkBlue);
    assert_eq!(sb.curr_line().unwrap().num_cells(), 80);
}

#[test]
fn erase_chars_triple_span() {
    use crossterm::style::Color;

    setup!(sb, parser, config);
    let fg = Color::from(&config.appearance.fg);
    drop(config);
    let mut driver = ScreenDriver::new(&mut sb);
    let seq = format!(
        "{}\n{}\nThis is span 1{}This is span 2{}This is span 3{}\n{}{}{}",
        WORDS[0],
        WORDS[1],
        csi!("32m"),
        csi!("34m"),
        csi!("0m"),
        csi!("F"),
        csi!("7C"),
        csi!("28X")
    );
    let parsed = parser.feed(seq.as_bytes());
    driver.process_events(parsed);

    assert_eq!(sb.cursor.x, 8);
    assert_eq!(sb.cursor.y, 3);
    assert_line_eq!(sb, 2, "This is                             span 3");
    assert_span_eq!(sb, 2, 0, expected => "This is       ", fg => fg);
    assert_span_eq!(sb, 2, 1, expected => "              ", fg => Color::DarkGreen);
    assert_span_eq!(sb, 2, 2, expected => "        span 3", fg => Color::DarkBlue);
    assert_eq!(sb.curr_line().unwrap().num_cells(), 80);
}

#[test]
fn delete_chars_double_span() {
    use crossterm::style::Color;

    setup!(sb, parser, config);
    {
        let fg = Color::from(&config.appearance.fg);
        drop(config);
        let mut driver = ScreenDriver::new(&mut sb);
        let seq = format!(
            "{}\n{}\nThis is span 1{}This is span 2{}\n{}{}{}",
            WORDS[0],
            WORDS[1],
            csi!("32m"),
            csi!("0m"),
            csi!("F"),
            csi!("7C"),
            csi!("14P")
        );
        let parsed = parser.feed(seq.as_bytes());
        driver.process_events(parsed);

        assert_eq!(sb.cursor.x, 8);
        assert_eq!(sb.cursor.y, 3);
        assert_line_eq!(sb, 2, "This is span 2");
        assert_span_eq!(sb, 2, 0, expected => "This is", fg => fg);
        assert_span_eq!(sb, 2, 1, expected => " span 2", fg => Color::DarkGreen);
        assert_eq!(sb.curr_line().unwrap().num_cells(), 80);
    }
    sb.lines.clear();
    sb.cursor = crate::screen::Position::ORIGIN;
    {
        let mut driver = ScreenDriver::new(&mut sb);
        let seq = format!(
            "{}\n{}\nThis is span 1{}This is span 2{}\n{}{}{}",
            WORDS[0],
            WORDS[1],
            csi!("32m"),
            csi!("0m"),
            csi!("F"),
            csi!("7C"),
            csi!("5P")
        );
        let parsed = parser.feed(seq.as_bytes());
        driver.process_events(parsed);

        assert_eq!(sb.cursor.x, 8);
        assert_eq!(sb.cursor.y, 3);
        assert_line_eq!(sb, 2, "This is 1This is span 2");
        assert_span_eq!(sb, 2, 0, expected => "This is 1");
        assert_span_eq!(sb, 2, 1, expected => "This is span 2");
        assert_eq!(sb.curr_line().unwrap().num_cells(), 80);
    }
}

#[test]
fn erase_chars_double_span() {
    use crossterm::style::Color;

    setup!(sb, parser, config);
    {
        let fg = Color::from(&config.appearance.fg);
        drop(config);
        let mut driver = ScreenDriver::new(&mut sb);
        let seq = format!(
            "{}\n{}\nThis is span 1{}This is span 2{}\n{}{}{}",
            WORDS[0],
            WORDS[1],
            csi!("32m"),
            csi!("0m"),
            csi!("F"),
            csi!("7C"),
            csi!("14X")
        );
        let parsed = parser.feed(seq.as_bytes());
        driver.process_events(parsed);

        assert_eq!(sb.cursor.x, 8);
        assert_eq!(sb.cursor.y, 3);
        assert_line_eq!(sb, 2, "This is               span 2");
        assert_span_eq!(sb, 2, 0, expected => "This is       ", fg => fg);
        assert_span_eq!(sb, 2, 1, expected => "        span 2", fg => Color::DarkGreen);
        assert_eq!(sb.curr_line().unwrap().num_cells(), 80);
    }
    sb.lines.clear();
    sb.cursor = crate::screen::Position::ORIGIN;
    {
        let mut driver = ScreenDriver::new(&mut sb);
        let seq = format!(
            "{}\n{}\nThis is span 1{}This is span 2{}\n{}{}{}",
            WORDS[0],
            WORDS[1],
            csi!("32m"),
            csi!("0m"),
            csi!("F"),
            csi!("7C"),
            csi!("5X")
        );
        let parsed = parser.feed(seq.as_bytes());
        driver.process_events(parsed);

        assert_eq!(sb.cursor.x, 8);
        assert_eq!(sb.cursor.y, 3);
        assert_line_eq!(sb, 2, "This is      1This is span 2");
        assert_span_eq!(sb, 2, 0, expected => "This is      1");
        assert_span_eq!(sb, 2, 1, expected => "This is span 2");
        assert_eq!(sb.curr_line().unwrap().num_cells(), 80);
    }
}

#[test]
fn delete_chars_single_span() {
    setup!(sb, parser);
    let mut driver = ScreenDriver::new(&mut sb);
    let seq = format!(
        "{}\n{}\n{}{}{}",
        WORDS[0],
        WORDS[1],
        csi!("F"),
        csi!("8C"),
        csi!("6P")
    );
    let parsed = parser.feed(seq.as_bytes());
    driver.process_events(parsed);

    assert_eq!(sb.cursor.x, 9);
    assert_eq!(sb.cursor.y, 2);
    assert_line_eq!(sb, 1, "This is nd line of words");
    assert_eq!(sb.curr_line().unwrap().num_cells(), 80);
}

#[test]
fn erase_chars_single_span() {
    setup!(sb, parser);
    let mut driver = ScreenDriver::new(&mut sb);
    let seq = format!(
        "{}\n{}\n{}{}{}",
        WORDS[0],
        WORDS[1],
        csi!("F"),
        csi!("8C"),
        csi!("6X")
    );
    let parsed = parser.feed(seq.as_bytes());
    driver.process_events(parsed);

    assert_eq!(sb.cursor.x, 9);
    assert_eq!(sb.cursor.y, 2);
    assert_line_eq!(sb, 1, "This is      ond line of words");
    assert_eq!(sb.curr_line().unwrap().num_cells(), 80);
}

#[test]
fn insert_delete_line() {
    setup!(sb, parser);
    let mut driver = ScreenDriver::new(&mut sb);
    let seq = format!(
        "{}\n{}{}Is this line inserted?\nCool now heres another line.\nLets add another.\nNow lets delete that last one.{}{}",
        WORDS[0],
        WORDS[1],
        csi!("L"),
        csi!("A"),
        csi!("M"),
    );
    let parsed = parser.feed(seq.as_bytes());
    driver.process_events(parsed);

    assert_line_eq!(sb, 1, "Is this line inserted?");
    assert_line_eq!(sb, 2, "Cool now heres another line.ds");
    assert_line_eq!(sb, 3, "Now lets delete that last one.");
}

#[test]
fn repeat_char() {
    setup!(sb, parser);
    let mut driver = ScreenDriver::new(&mut sb);
    let seq = format!("{}\n{}{}", WORDS[0], WORDS[1], csi!("10b"),);
    let parsed = parser.feed(seq.as_bytes());
    driver.process_events(parsed);

    assert_line_eq!(sb, 0, "This is one line of words");
    assert_line_eq!(sb, 1, "This is a second line of wordsssssssssss");
}
