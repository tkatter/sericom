#![allow(unused)]
use crossterm::style::{Attributes, Color, Colors};
use std::io::BufWriter;
use tracing::instrument;

use super::{Cursor, ScreenBuffer, UIAction};
use crate::{
    configs::get_config,
    ui::{ByteParser, Cell, Line, NL, ParserEvent, Span},
};

const MIN_RENDER_INTERVAL: tokio::time::Duration = tokio::time::Duration::from_millis(33);

impl ScreenBuffer {
    /// Takes incoming data (bytes (`u8`) from a serial connection) and
    /// processes them accordingly, handling ascii escape sequences, to
    /// render as characters/strings in the terminal.
    #[instrument(name = "Add Data", skip(self, data))]
    pub fn add_data(&mut self, parser: &mut ByteParser, data: &[u8]) {
        let events = parser.feed(data);
        // self.process_events(writer, event);
    }

    // fn add_char_batch(&mut self, chars: &[char]) {
    //     tracing::debug!("CharBatch: '{:?}'", chars);
    //     while self.cursor.y >= self.lines.len() {
    //         self.lines.push_back(Line::new_default(self.width.into()));
    //     }
    //
    //     if let Some(line) = self.lines.get_mut(self.cursor.y) {
    //         for &ch in chars {
    //             line.set_char(self.cursor.x as usize, ch);
    //             self.cursor.x += 1;
    //             if self.cursor.x >= self.width {
    //                 self.new_line();
    //                 break;
    //             }
    //         }
    //     }
    // }

    /// A helper function to check whether the terminal's screen should be rendered.
    pub fn should_render_now(&self) -> bool {
        // use tokio::time::Instant;
        //
        // if !self.needs_render {
        //     return false;
        // }
        //
        // let now = Instant::now();
        // match self.last_render {
        //     Some(last) => now.duration_since(last) >= MIN_RENDER_INTERVAL,
        //     None => true,
        // }
        true
    }

    /// Writes the lines/characters received from `add_data` to the terminal's screen.
    ///
    /// As of now, `render` does not involve any diff-ing of previous renders.
    /// The nature of communicating to devices over a serial connection is similar
    /// that of a terminal; lines get printed to a screen and with each new line,
    /// all of the previously rendered characters must be re-rendered one cell higher.
    ///
    /// Because of this, the only diff-ing that would make sense would be
    /// that of the cells within the screen that are simply blank.
    #[allow(clippy::similar_names)]
    pub fn render(&mut self) -> std::io::Result<()> {
        //     use crossterm::{cursor, queue, style};
        //     use std::io::{self, Write};
        //     use tokio::time::Instant;
        //
        //     if !self.needs_render {
        //         return Ok(());
        //     }
        //
        //     let mut writer = BufWriter::new(io::stdout());
        //     queue!(writer, cursor::Hide)?;
        //     let config = get_config();
        //
        //     for screen_y in 0..self.height {
        //         let line_idx = self.view_start + screen_y as usize;
        //         queue!(writer, cursor::MoveTo(0, screen_y))?;
        //
        //         if let Some(line) = self.lines.get_mut(line_idx) {
        //             let mut current_fg = Color::from(&config.appearance.fg);
        //             let mut current_bg = Color::from(&config.appearance.bg);
        //             queue!(
        //                 writer,
        //                 style::SetForegroundColor(current_fg),
        //                 style::SetBackgroundColor(current_bg)
        //             )?;
        //
        //             for cell in line {
        //                 let global_reverse = self.display_attributes.has(style::Attribute::Reverse);
        //
        //                 let fg = if (cell.is_selected && !global_reverse)
        //                     || (!cell.is_selected && global_reverse)
        //                 {
        //                     cell.bg_color
        //                 } else {
        //                     cell.fg_color
        //                 };
        //
        //                 let bg = if (cell.is_selected && !global_reverse)
        //                     || (!cell.is_selected && global_reverse)
        //                 {
        //                     cell.fg_color
        //                 } else {
        //                     cell.bg_color
        //                 };
        //
        //                 if fg != current_fg {
        //                     queue!(writer, style::SetForegroundColor(fg))?;
        //                     current_fg = fg;
        //                 }
        //                 if bg != current_bg {
        //                     queue!(writer, style::SetBackgroundColor(bg))?;
        //                     current_bg = bg;
        //                 }
        //
        //                 if self.display_attributes.has(style::Attribute::Bold) {
        //                     queue!(
        //                         writer,
        //                         style::SetAttribute(style::Attribute::Bold),
        //                         style::Print(cell.character)
        //                     )?;
        //                 } else {
        //                     queue!(writer, style::Print(cell.character))?;
        //                 }
        //             }
        //         } else {
        //             queue!(
        //                 writer,
        //                 style::ResetColor,
        //                 style::Print(" ".repeat(self.width as usize))
        //             )?;
        //         }
        //     }
        //
        //     // This is relative the the terminal's L x W, whereas
        //     // self.cursor_pos.y is within the entire line buf
        //     let screen_cursor_y = if self.cursor.y >= self.view_start
        //         && self.cursor.y < self.view_start + self.height as usize
        //     {
        //         (self.cursor.y - self.view_start) as u16
        //     } else {
        //         self.height - 1
        //     };
        //
        //     queue!(
        //         writer,
        //         cursor::MoveTo(self.cursor.x, screen_cursor_y),
        //         cursor::Show
        //     )?;
        //     writer.flush()?;
        //
        //     self.last_render = Some(Instant::now());
        //     self.needs_render = false;
        Ok(())
    }
}
