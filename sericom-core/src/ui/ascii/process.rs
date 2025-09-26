use std::io::Write;

use crossterm::{
    cursor, queue,
    style::{Attribute, Print, SetAttribute},
    terminal::{Clear, ClearType},
};
use miette::IntoDiagnostic;

use super::*;
use crate::{
    screen_buffer::ScreenBuffer,
    ui::{Cell, Cursor, Line, Span, line::ColorState},
};

impl ScreenBuffer {
    pub fn process_events<W: Write>(&mut self, events: Vec<ParserEvent>) -> miette::Result<()> {
        let mut color_state = ColorState::default();

        let mut curr_span = Span::default();
        let mut curr_line = Line::new_default(40);

        for ev in events {
            match &ev {
                ParserEvent::Text(t) => {
                    t.iter().for_each(|c| {
                        curr_span.push(Cell::new(char::from(*c)));
                    });
                }
                ParserEvent::Control(b) => {
                    match *b {
                        BS => self.move_cursor_left(1),
                        CR => self.set_cursor_col(0),
                        // Need to handle creating new empty buffer
                        FF => todo!(),
                        // FF => queue!(writer, Clear(ClearType::All)).into_diagnostic()?,
                        NL => {
                            if !curr_span.is_empty() {
                                curr_line.push(std::mem::take(&mut curr_span));
                            }
                            self.push_line(std::mem::take(&mut curr_line));
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
                        && *seq.last().unwrap() == b'm'
                    {
                        extract_color_seq(writer, &mut color_state, seq)?;
                    }
                }
            }
        }
        writer.flush().ok();
        Ok(())
    }
}

fn extract_color_seq<W: Write>(
    writer: &mut W,
    color_state: &mut ColorState,
    seq: &[u8],
) -> miette::Result<()> {
    // Get the part between 'ESC[' and 'm'
    let body = &seq[2..seq.len() - 1];

    // If none, it is just setting a graphics mode
    let Some(first_part_idx) = body.iter().position(|&b| b == SEP) else {
        process_graphics_mode(writer, color_state, body)?;
        return Ok(());
    };

    let (mode, color_bytes) = body.split_at(first_part_idx);

    // If true, color_str skips the graphics_mode part
    let color_str = if process_graphics_mode(writer, color_state, mode)? {
        &color_bytes[1..]
    } else {
        body
    };

    if let Ok(color_str) = str::from_utf8(color_str) {
        color_state.set_colors(color_str);
    }

    Ok(())
}

fn process_graphics_mode<W: Write>(
    writer: &mut W,
    color_state: &mut ColorState,
    body: &[u8],
) -> miette::Result<bool> {
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
            return Ok(false);
        } // ignore the rest
    };
    if *body == [b'2', b'2'] {
        queue!(
            writer,
            SetAttribute(attr),
            SetAttribute(Attribute::NormalIntensity)
        )
        .into_diagnostic()?;
    } else {
        queue!(writer, SetAttribute(attr)).into_diagnostic()?;
    }
    Ok(true)
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
