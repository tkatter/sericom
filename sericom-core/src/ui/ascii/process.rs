use crossterm::style::{Attribute, Attributes, Color};

use crate::{
    screen_buffer::ScreenBuffer,
    ui::{
        BK, BS, CR, Cell, Cursor, ESC, FF, Line, NL, ParserEvent, Rect, SEP, Span, TAB,
        line::ColorState,
    },
};

impl ScreenBuffer {
    pub(crate) fn process_events(&mut self, events: Vec<ParserEvent>) {
        let mut color_state = ColorState::default();
        let mut attrs = Attributes::default();

        let mut curr_line = Line::reserve_new(usize::from(self.rect.width));

        let span_cap = |line: &Line, rect: &Rect| -> usize {
            if !line.is_empty() {
                return usize::from(rect.width) - line.num_cells();
            }
            usize::from(rect.width)
        };

        let mut curr_span = Span::reserve_new(span_cap(&curr_line, &self.rect), None, None);

        for ev in events {
            match &ev {
                ParserEvent::Text(t) => {
                    for c in t {
                        let total_cells = curr_line.num_cells() + curr_span.len();

                        // Don't want an else branch because don't want line wrap
                        if total_cells < usize::from(self.rect.width) {
                            curr_span.push(Cell::new(char::from(*c)));
                        }
                    }
                }
                ParserEvent::Control(b) => {
                    match *b {
                        BS => self.move_cursor_left(1),
                        CR => self.set_cursor_col(0),
                        // Need to handle creating new empty buffer
                        FF => todo!(),
                        // FF => queue!(writer, Clear(ClearType::All)).into_diagnostic()?,
                        NL => {
                            let remainder = usize::from(self.rect.width)
                                - (curr_span.len() + curr_line.num_cells());
                            if remainder != 0 {
                                curr_span.fill_to_width(span_cap(&curr_line, &self.rect));
                                curr_line.push(curr_span);
                                curr_span = Span::reserve_new(
                                    span_cap(&curr_line, &self.rect),
                                    Some(color_state.get_colors()),
                                    Some(attrs),
                                );
                            }
                            self.push_line(curr_line);
                            curr_line = Line::reserve_new(usize::from(self.rect.width));
                        }
                        TAB => todo!(),
                        _ => {}
                    }
                }
                ParserEvent::EscapeSequence(seq) => {
                    // Verify it is a color sequence 'ESC[_m'
                    if seq.len() >= 3
                        && seq[0] == ESC
                        && seq[1] == BK
                        && *seq.last().expect("Verified len != 0") == b'm'
                    {
                        // Start a new span for change in graphics
                        if !curr_span.is_empty() {
                            curr_span.shrink();
                            curr_line.push(curr_span);
                            curr_span =
                                Span::reserve_new(span_cap(&curr_line, &self.rect), None, None);
                        }
                        extract_color_seq(&mut color_state, &mut attrs, seq);
                        curr_span.set_colors(&color_state);
                        curr_span.set_attrs(attrs);
                    }
                }
            }
        }
    }
}

fn extract_color_seq(color_state: &mut ColorState, attrs: &mut Attributes, seq: &[u8]) {
    // Get the part between 'ESC[' and 'm'
    let body = &seq[2..seq.len() - 1];
    eprintln!("BODY: {}", String::from_utf8_lossy(body));

    // If none, it is just setting a graphics mode
    let Some(first_part_idx) = body.iter().position(|&b| b == SEP) else {
        process_graphics_mode(color_state, attrs, body);
        eprintln!("SHORT-CIRCUIT-EXTRACTED: {}", String::from_utf8_lossy(body));
        return;
    };

    let (mode, color_bytes) = body.split_at(first_part_idx);
    eprintln!(
        "MODE: {}, COLOR_BYTES: {}",
        String::from_utf8_lossy(mode),
        String::from_utf8_lossy(color_bytes)
    );

    // If true, color_str skips the graphics_mode part
    let color_str = if process_graphics_mode(color_state, attrs, mode) {
        &color_bytes[1..]
    } else {
        body
    };

    if let Ok(color_str) = str::from_utf8(color_str) {
        color_state.set_colors(color_str);
    }
}

fn process_graphics_mode(
    color_state: &mut ColorState,
    attrs: &mut Attributes,
    body: &[u8],
) -> bool {
    let attr = match *body {
        [b'0'] => {
            *color_state = ColorState::default();
            Attribute::Reset
        }
        [b'1'] => Attribute::Bold,
        [b'2'] => Attribute::Dim,
        [b'3'] => Attribute::Italic,
        [b'4'] => Attribute::Underlined,
        [b'5'] => Attribute::SlowBlink,
        [b'7'] => Attribute::Reverse,
        [b'8'] => Attribute::Hidden,
        [b'9'] => Attribute::CrossedOut,
        [b'2', b'2'] => Attribute::NoBold,
        [b'2', b'3'] => Attribute::NoItalic,
        [b'2', b'4'] => Attribute::NoUnderline,
        [b'2', b'5'] => Attribute::NoBlink,
        [b'2', b'7'] => Attribute::NoReverse,
        [b'2', b'8'] => Attribute::NoHidden,
        [b'2', b'9'] => Attribute::NotCrossedOut,
        _ => {
            // try parsing foreground colors i.e. ESC[34m
            // if let Ok(ascii_str) = str::from_utf8(body) {
            //     eprintln!("ASCII_STR: {}", ascii_str);
            //     color_state.set_colors(ascii_str);
            // }
            // handle colors like "30".."37", "40".."47", "90".."97", "100".."107"
            if let Ok(s) = std::str::from_utf8(body) {
                if let Ok(num) = s.parse::<u8>() {
                    match num {
                        // basic FG
                        30 => color_state.set_fg(Color::Black),
                        31 => color_state.set_fg(Color::DarkRed),
                        32 => color_state.set_fg(Color::DarkGreen),
                        33 => color_state.set_fg(Color::DarkYellow),
                        34 => color_state.set_fg(Color::DarkBlue),
                        35 => color_state.set_fg(Color::DarkMagenta),
                        36 => color_state.set_fg(Color::DarkCyan),
                        37 => color_state.set_fg(Color::Grey),
                        // basic BG
                        40 => color_state.set_bg(Color::Black),
                        41 => color_state.set_bg(Color::DarkRed),
                        42 => color_state.set_bg(Color::DarkGreen),
                        43 => color_state.set_bg(Color::DarkYellow),
                        44 => color_state.set_bg(Color::DarkBlue),
                        45 => color_state.set_bg(Color::DarkMagenta),
                        46 => color_state.set_bg(Color::DarkCyan),
                        47 => color_state.set_bg(Color::Grey),
                        // bright FG
                        90 => color_state.set_fg(Color::DarkGrey),
                        91 => color_state.set_fg(Color::Red),
                        92 => color_state.set_fg(Color::Green),
                        93 => color_state.set_fg(Color::Yellow),
                        94 => color_state.set_fg(Color::Blue),
                        95 => color_state.set_fg(Color::Magenta),
                        96 => color_state.set_fg(Color::Cyan),
                        97 => color_state.set_fg(Color::White),
                        // bright BG
                        100 => color_state.set_bg(Color::DarkGrey),
                        101 => color_state.set_bg(Color::Red),
                        102 => color_state.set_bg(Color::Green),
                        103 => color_state.set_bg(Color::Yellow),
                        104 => color_state.set_bg(Color::Blue),
                        105 => color_state.set_bg(Color::Magenta),
                        106 => color_state.set_bg(Color::Cyan),
                        107 => color_state.set_bg(Color::White),
                        _ => {}
                    }
                }
            }
            return false;
        } // ignore the rest
    };
    if *body == [b'2', b'2'] {
        attrs.set(Attribute::NormalIntensity);
    }
    attrs.set(attr);
    true
}
// pub fn process_events<W: Write>(
//     writer: &mut W,
//     events: Vec<ParserEvent>,
// ) -> miette::Result<()> {
//     let mut color_state = ColorState::default();
//     for ev in events {
//         match &ev {
//             ParserEvent::Text(t) => {
//                 if let Ok(s) = std::str::from_utf8(t) {
//                     color_state.queue_line(writer, s)?;
//                 }
//             }
//             ParserEvent::Control(b) => {
//                 match *b {
//                     BS => queue!(writer, cursor::MoveLeft(1)).into_diagnostic()?,
//                     CR => queue!(writer, cursor::MoveToColumn(0)).into_diagnostic()?,
//                     // Need to handle creating new empty buffer
//                     FF => queue!(writer, Clear(ClearType::All)).into_diagnostic()?,
//                     NL => queue!(writer, Print("\n")).into_diagnostic()?,
//                     TAB => queue!(writer, Print("\t")).into_diagnostic()?,
//                     _ => {}
//                 }
//             }
//             ParserEvent::EscapeSequence(seq) => {
//                 // Verify it is a color sequence 'ESC[_m'
//                 if seq.len() >= 3 && seq[0] == 0x1B && seq[1] == b'[' && *seq.last().unwrap() == b'm' {
//                     extract_color_seq(writer, &mut color_state, seq)?;
//                 }
//             }
//         }
//     }
//     writer.flush().ok();
//     Ok(())
// }
