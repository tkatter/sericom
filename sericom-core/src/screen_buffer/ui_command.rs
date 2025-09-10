use super::{Cursor, Line, ScreenBuffer};

/// `UICommand` is used for communication between stdin and the [`ScreenBuffer`].
#[non_exhaustive]
#[derive(Clone, Debug)]
pub enum UICommand {
    ScrollUp(usize),
    ScrollDown(usize),
    ScrollBottom,
    ScrollTop,
    StartSelection(u16, u16),
    UpdateSelection(u16, u16),
    CopySelection,
    ClearBuffer,
}

pub(crate) trait UIAction {
    fn scroll_up(&mut self, lines: usize);
    fn scroll_down(&mut self, lines: usize);
    fn scroll_to_bottom(&mut self);
    fn scroll_to_top(&mut self);
    fn start_selection(&mut self, screen_x: u16, screen_y: u16);
    fn update_selection(&mut self, screen_x: u16, screen_y: u16);
    fn clear_selection(&mut self);
    fn copy_to_clipboard(&mut self) -> std::io::Result<()>;
    fn clear_buffer(&mut self);
    fn clear_screen(&mut self);
}

impl UIAction for ScreenBuffer {
    /// Called to scroll the terminal up by `lines`.
    fn scroll_up(&mut self, lines: usize) {
        if self.view_start >= lines {
            self.view_start -= lines;
        } else {
            self.view_start = 0;
        }
        self.clear_selection();
        self.needs_render = true;
    }

    /// Called to scroll the terminal down by `lines`.
    fn scroll_down(&mut self, lines: usize) {
        let max_view_start = self.lines.len().saturating_sub(self.height as usize);
        self.view_start = (self.view_start + lines).min(max_view_start);
        self.clear_selection();
        self.needs_render = true;
    }

    /// Scrolls to the bottom of the screen. The bottom of the screen is
    /// the same as the most recent lines received from the serial connection
    fn scroll_to_bottom(&mut self) {
        self.view_start = self.lines.len().saturating_sub(self.height as usize);
        self.needs_render = true;
    }

    /// Scrolls to the top of the serial connection's history.
    fn scroll_to_top(&mut self) {
        self.view_start = 0;
        self.needs_render = true;
    }

    /// Sets the position within the screen for the start of a selection.
    /// Where `screen_x` is the x-position of the start of the selection,
    /// and `screen_y` is the y-position (line) of the start of the selection.
    fn start_selection(&mut self, screen_x: u16, screen_y: u16) {
        let absolute_line = self.view_start + screen_y as usize;
        self.clear_selection();
        self.selection_start = Some((screen_x, absolute_line));
        self.needs_render = true;
    }

    /// Update's a selection to include the position passed to it.
    /// Where `screen_x` is the x-position and `screen_y` is the y-position (line).
    fn update_selection(&mut self, screen_x: u16, screen_y: u16) {
        let absolute_line = self.view_start + screen_y as usize;
        self.selection_end = Some((screen_x, absolute_line));
        self.update_selection_highlighting();
        self.needs_render = true;
    }

    /// Clears the selection state.
    fn clear_selection(&mut self) {
        for line in &mut self.lines {
            line.clear_selection();
        }
        self.selection_start = None;
        self.selection_end = None;
        self.needs_render = true;
    }

    /// Copy's the currently selected text to the user's clipboard.
    fn copy_to_clipboard(&mut self) -> std::io::Result<()> {
        use crossterm::{clipboard, execute};

        let selected_text = self.get_selected_text();
        if !selected_text.is_empty() {
            execute!(
                std::io::stdout(),
                clipboard::CopyToClipboard::to_clipboard_from(selected_text)
            )?;
        }
        self.clear_selection();
        Ok(())
    }

    /// Clears the entire serial connection's history and reset's the screen.
    /// Similar to <kbd>Ctrl</kbd> + <kbd>Shift</kbd> + <kbd>l</kbd> in a terminal, except this will reset the
    /// connection's message history (on the user's side).
    fn clear_buffer(&mut self) {
        self.lines.clear();
        self.view_start = 0;
        self.set_cursor_pos((0_u16, 0_usize));
        self.lines.push_back(Line::new(self.width as usize));
        self.needs_render = true;
    }

    /// Clears the current screen while keeping the buffer's history
    fn clear_screen(&mut self) {
        for _ in 0..self.height {
            self.lines.push_back(Line::new(self.width as usize));
        }
        self.view_start = self.lines.len() - self.height as usize;
        self.needs_render = true;
    }
}

impl ScreenBuffer {
    fn update_selection_highlighting(&mut self) {
        for line in &mut self.lines {
            line.clear_selection();
        }

        if let (Some((start_x, start_line)), Some((end_x, end_line))) =
            (self.selection_start, self.selection_end)
        {
            let (start_line, start_x, end_line, end_x) =
                if start_line < end_line || (start_line == end_line && start_x <= end_x) {
                    (start_line, start_x, end_line, end_x)
                } else {
                    (end_line, end_x, start_line, start_x)
                };

            for line_idx in start_line..=end_line {
                if let Some(line) = self.lines.get_mut(line_idx) {
                    let line_start_x = if line_idx == start_line { start_x } else { 0 };
                    let line_end_x = if line_idx == end_line {
                        end_x
                    } else {
                        self.width - 1
                    };

                    for x in line_start_x..=line_end_x.min(self.width - 1) {
                        if let Some(cell) = line.get_mut_cell(x as usize) {
                            cell.is_selected = true;
                        }
                    }
                }
            }
        }
    }

    fn get_selected_text(&self) -> String {
        if let (Some((start_x, start_line)), Some((end_x, end_line))) =
            (self.selection_start, self.selection_end)
        {
            let (start_line, start_x, end_line, end_x) =
                if start_line < end_line || (start_line == end_line && start_x <= end_x) {
                    (start_line, start_x, end_line, end_x)
                } else {
                    (end_line, end_x, start_line, start_x)
                };

            let mut result = String::new();

            for line_idx in start_line..=end_line {
                if let Some(line) = self.lines.get(line_idx) {
                    let line_start_x = if line_idx == start_line { start_x } else { 0 };
                    let line_end_x = if line_idx == end_line {
                        end_x
                    } else {
                        self.width - 1
                    };

                    for x in line_start_x..=line_end_x.min(self.width - 1) {
                        if let Some(cell) = line.get_cell(x as usize) {
                            result.push(cell.character);
                        }
                    }

                    if line_idx < end_line {
                        result.push('\n');
                    }
                }
            }

            result.trim_end().to_string()
        } else {
            String::new()
        }
    }
}
