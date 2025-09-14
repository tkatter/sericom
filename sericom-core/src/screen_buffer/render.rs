use crossterm::style::Color;
use std::io::BufWriter;
use tracing::instrument;

use super::{Cursor, EscapeState, Line, ScreenBuffer, UIAction};
use crate::configs::get_config;

const MIN_RENDER_INTERVAL: tokio::time::Duration = tokio::time::Duration::from_millis(33);

impl ScreenBuffer {
    /// Takes incoming data (bytes (`u8`) from a serial connection) and
    /// processes them accordingly, handling ascii escape sequences, to
    /// render as characters/strings in the terminal.
    #[allow(clippy::missing_panics_doc)]
    #[allow(clippy::cast_possible_truncation)]
    #[instrument(name = "Add Data", skip(self, data))]
    pub fn add_data(&mut self, data: &[u8]) {
        let text = String::from_utf8_lossy(data);
        let mut chars = text.chars().peekable();

        while let Some(ch) = chars.next() {
            match self.escape_state {
                EscapeState::Normal => {
                    match ch {
                        '\r' => {
                            self.cursor_pos.x = 0;
                            if chars.peek() == Some(&'\n') {
                                chars.next();
                                self.new_line();
                            }
                        }
                        '\n' => {
                            self.new_line();
                        }
                        '\x07' | '\x0E' | '\x0F' => {}
                        '\x08' => {
                            #[allow(clippy::unused_peekable)]
                            let mut temp_chars = chars.clone();
                            // Matches the `\x08 ' ' \x08` deletion sequence
                            if (Some(' '), Some('\x08')) == (temp_chars.next(), temp_chars.next()) {
                                // Consume them - to remove from further processing
                                chars.next();
                                chars.next();
                                self.move_cursor_left(1);
                                self.set_char_at_cursor(' ');
                            } else {
                                // If not the deletion sequence, move cursor left
                                // when receiving a single '\x08'
                                self.move_cursor_left(1);
                            }
                        }
                        '\x1B' => self.escape_state = EscapeState::Esc,
                        c => {
                            let mut batch = vec![c];
                            while let Some(&next_ch) = chars.peek() {
                                if next_ch.is_control()
                                    || next_ch == '\x1B'
                                    || self.cursor_pos.x + batch.len() as u16 >= self.width
                                {
                                    break;
                                }
                                batch.push(chars.next().unwrap());
                            }
                            self.add_char_batch(&batch);
                        }
                    }
                }
                EscapeState::Esc => match ch {
                    '[' => self.escape_state = EscapeState::Csi,
                    _ => self.escape_state = EscapeState::Normal,
                },
                EscapeState::Csi => match ch {
                    ';' => self.escape_sequence.insert_separator(),
                    c if ch.is_ascii_digit() => self.escape_sequence.push_num(c),
                    c if c.is_ascii_alphabetic() => {
                        // Reset because actions are the last members of a sequence
                        self.escape_sequence.push_action(c);
                        self.parse_sequence();
                        self.escape_sequence.reset();
                        self.escape_state = EscapeState::Normal;
                    }
                    // NOTE: May need to handle '?', ':', and '>'
                    _ => self.escape_state = EscapeState::Normal,
                },
            }
        }
        // Sets `self.needs_render = true`
        self.scroll_to_bottom();
    }

    fn add_char_batch(&mut self, chars: &[char]) {
        tracing::debug!("CharBatch: '{:?}'", chars);
        while self.cursor_pos.y >= self.lines.len() {
            self.lines.push_back(Line::new_default(self.width as usize));
        }

        if let Some(line) = self.lines.get_mut(self.cursor_pos.y) {
            for &ch in chars {
                line.set_char(self.cursor_pos.x as usize, ch);
                self.cursor_pos.x += 1;
                if self.cursor_pos.x >= self.width {
                    self.new_line();
                    break;
                }
            }
        }
    }

    /// A helper function to check whether the terminal's screen should be rendered.
    #[must_use]
    pub fn should_render_now(&self) -> bool {
        use tokio::time::Instant;

        if !self.needs_render {
            return false;
        }

        let now = Instant::now();
        self.last_render
            .is_none_or(|last| now.duration_since(last) >= MIN_RENDER_INTERVAL)
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
    ///
    /// # Errors
    /// This function will only error if [`queue`] errors
    /// when drawing to the terminal.
    ///
    /// [`queue`]: crossterm::queue
    #[allow(clippy::similar_names)]
    #[allow(clippy::cast_possible_truncation)]
    pub fn render(&mut self) -> std::io::Result<()> {
        use crossterm::{cursor, queue, style};
        use std::io::{self, Write};
        use tokio::time::Instant;

        if !self.needs_render {
            return Ok(());
        }

        let mut writer = BufWriter::new(io::stdout());
        queue!(writer, cursor::Hide)?;
        let config = get_config();

        for screen_y in 0..self.height {
            let line_idx = self.view_start + screen_y as usize;
            queue!(writer, cursor::MoveTo(0, screen_y))?;

            if let Some(line) = self.lines.get_mut(line_idx) {
                let mut current_fg = Color::from(&config.appearance.fg);
                let mut current_bg = Color::from(&config.appearance.bg);
                queue!(
                    writer,
                    style::SetForegroundColor(current_fg),
                    style::SetBackgroundColor(current_bg)
                )?;

                for cell in line {
                    let global_reverse = self.display_attributes.has(style::Attribute::Reverse);

                    let fg = if (cell.is_selected && !global_reverse)
                        || (!cell.is_selected && global_reverse)
                    {
                        cell.bg_color
                    } else {
                        cell.fg_color
                    };

                    let bg = if (cell.is_selected && !global_reverse)
                        || (!cell.is_selected && global_reverse)
                    {
                        cell.fg_color
                    } else {
                        cell.bg_color
                    };

                    if fg != current_fg {
                        queue!(writer, style::SetForegroundColor(fg))?;
                        current_fg = fg;
                    }
                    if bg != current_bg {
                        queue!(writer, style::SetBackgroundColor(bg))?;
                        current_bg = bg;
                    }

                    if self.display_attributes.has(style::Attribute::Bold) {
                        queue!(
                            writer,
                            style::SetAttribute(style::Attribute::Bold),
                            style::Print(cell.character)
                        )?;
                    } else {
                        queue!(writer, style::Print(cell.character))?;
                    }
                }
            } else {
                queue!(
                    writer,
                    style::ResetColor,
                    style::Print(" ".repeat(self.width as usize))
                )?;
            }
        }

        // This is relative the the terminal's L x W, whereas
        // self.cursor_pos.y is within the entire line buf
        let screen_cursor_y = if self.cursor_pos.y >= self.view_start
            && self.cursor_pos.y < self.view_start + self.height as usize
        {
            (self.cursor_pos.y - self.view_start) as u16
        } else {
            self.height - 1
        };

        queue!(
            writer,
            cursor::MoveTo(self.cursor_pos.x, screen_cursor_y),
            cursor::Show
        )?;
        writer.flush()?;

        self.last_render = Some(Instant::now());
        self.needs_render = false;
        Ok(())
    }
}
