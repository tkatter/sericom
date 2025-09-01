//! This module contains the code needed for the implementation of a
//! stateful buffer that holds a history of the lines/data received
//! from the serial connection and the rendering/updating of the buffer
//! to the terminal screen (stdout).
//!
//! Simply writing the data received from the serial connection directly
//! to stdout creates one main issue: there is no history of previous lines
//! that were received from the serial connection. Without a screen buffer,
//! lines would simply be wiped from existence as they exit the terminal's screen.
//!
//! As a result, there would be no way to implement features like scrolling,
//! highlighting text (for UI purposes), and getting characters at specific
//! locations within the screen for things like copying to a clipboard.
//!
//! The screen buffer solves these issues by storing each line received from the
//! connection in a [`VecDeque`]. It is important to note that
//! currently, the **capacity of the `VecDeque` is not hardcoded and is theoretically
//! allowed to grow forever**, limited by memory.
use crate::configs::get_config;
use crossterm::style::Color;
use std::{collections::VecDeque, io::BufWriter};

const MIN_RENDER_INTERVAL: tokio::time::Duration = tokio::time::Duration::from_millis(33);

/// `EscapeState` holds stateful information about the incoming
/// data to allow for proper processing of ansii escape codes/characters.
enum EscapeState {
    /// Has not received ansii escape characters
    Normal,
    /// Just received an ESC (0x1B)
    Esc,
    /// Received ESC and then '[' (0x5B)
    Csi,
}

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

/// `Cell` represents a cell within the terminal's window/frame.
/// Used to hold rendering state for all the cells within the [`ScreenBuffer`].
/// Each line within `ScreenBuffer` is represented by a `Vec<Cell>`.
#[derive(Clone, Debug)]
struct Cell {
    character: char,
    fg_color: Color,
    bg_color: Color,
    is_selected: bool,
}

impl Default for Cell {
    fn default() -> Self {
        let config = get_config();
        Self {
            character: ' ',
            fg_color: Color::from(&config.appearance.fg),
            bg_color: Color::from(&config.appearance.bg),
            is_selected: false,
        }
    }
}

/// Represent's the cursor's position within the [`ScreenBuffer`].
#[derive(Clone, Copy, Debug)]
struct Position {
    /// The x position of a line within `ScreenBuffer`'s scrollback buffer.
    /// This translates to the `Cell` within a line (`Vec`).
    x: u16,
    /// `y` is the line number within `ScreenBuffer`'s scrollback buffer.
    y: usize,
}

impl From<(u16, usize)> for Position {
    fn from((x, y): (u16, usize)) -> Self {
        Self { x, y }
    }
}

impl From<(u16, u16)> for Position {
    fn from((x, y): (u16, u16)) -> Self {
        Self { x, y: y as usize }
    }
}

impl From<Position> for (u16, usize) {
    fn from(position: Position) -> Self {
        (position.x, position.y)
    }
}

impl From<Position> for (u16, u16) {
    fn from(position: Position) -> Self {
        (position.x, position.y as u16)
    }
}

/// The `ScreenBuffer` holds rendering state for the entire terminal's window/frame.
/// It mainly serves to allow for user-interactions that require a history and location
/// of the data displayed within the terminal i.e. copy/paste, scrolling, & highlighting.
pub struct ScreenBuffer {
    /// Terminal width
    width: u16,
    /// Terminal height
    height: u16,
    /// Scrollback buffer (all lines received from the serial connection).
    /// Limited by memory.
    lines: VecDeque<Vec<Cell>>,
    /// Current view into the buffer.
    /// Denotes which line is at the top of the screen.
    view_start: usize,
    /// Position of the cursor within the `ScreenBuffer`.
    cursor_pos: Position,
    /// Start of text selection. Used for highlighting and copying to clipboard.
    selection_start: Option<(u16, usize)>,
    /// End of text selection. Used for highlighting and copying to clipboard.
    selection_end: Option<(u16, usize)>,
    /// Configuration for the maximum amount of lines to keep in memory.
    max_scrollback: usize,
    /// Represents the current state for handling ansii escape sequences
    /// as incoming data is being processed.
    escape_state: EscapeState,
    last_render: Option<tokio::time::Instant>,
    needs_render: bool,
}

impl ScreenBuffer {
    /// Constructs a new `ScreenBuffer` to be used at the start of a
    /// serial connection session.
    pub fn new(width: u16, height: u16, max_scrollback: usize) -> Self {
        let mut buffer = Self {
            width,
            height,
            lines: VecDeque::new(),
            view_start: 0,
            cursor_pos: Position { x: 0, y: 0 },
            selection_start: None,
            selection_end: None,
            max_scrollback,
            last_render: None,
            needs_render: false,
            escape_state: EscapeState::Normal,
        };
        // Start with an empty line
        buffer
            .lines
            .push_back(vec![Cell::default(); width as usize]);
        buffer
    }

    fn set_cursor_pos<P: Into<Position>>(&mut self, position: P) {
        self.cursor_pos = position.into();
    }

    fn move_cursor_left(&mut self) {
        self.cursor_pos.x = self.cursor_pos.x.saturating_sub(1);
    }

    fn move_cursor_right(&mut self) {
        self.cursor_pos.x = self.cursor_pos.x.saturating_add(1);
    }

    /// Takes incoming data (bytes (`u8`) from a serial connection) and
    /// processes them accordingly, handling ascii escape sequences, to
    /// render as characters/strings in the terminal.
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
                        '\x07' => {}
                        '\x08' => {
                            let mut temp_chars = chars.clone();
                            // Matches the `\x08 ' ' \x08` deletion sequence
                            if let (Some(' '), Some('\x08')) =
                                (temp_chars.next(), temp_chars.next())
                            {
                                // Consume them - to remove from further processing
                                chars.next();
                                chars.next();
                                self.move_cursor_left();
                                self.set_char_at_cursor(' ');
                            } else {
                                // If not the deletion sequence, move cursor left
                                // when receiving a single '\x08'
                                self.move_cursor_left();
                            }
                        }
                        '\x1B' => {
                            self.escape_state = EscapeState::Esc;
                        }
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
                    '[' => {
                        self.escape_state = EscapeState::Csi;
                    }
                    _ => {
                        self.escape_state = EscapeState::Normal;
                    }
                },
                EscapeState::Csi => match ch {
                    'J' => {
                        self.clear_from_cursor_to_eol();
                        self.escape_state = EscapeState::Normal;
                    }
                    'K' => {
                        self.clear_from_cursor_to_eol();
                        self.escape_state = EscapeState::Normal;
                    }
                    'C' => {
                        self.move_cursor_left();
                        self.escape_state = EscapeState::Normal;
                    }
                    'D' => {
                        self.move_cursor_right();
                        self.escape_state = EscapeState::Normal;
                    }
                    _ => {
                        self.escape_state = EscapeState::Normal;
                    }
                },
            }
        }
        self.scroll_to_bottom();
        self.needs_render = true;
    }

    fn add_char_batch(&mut self, chars: &[char]) {
        while self.cursor_pos.y >= self.lines.len() {
            self.lines
                .push_back(vec![Cell::default(); self.width as usize]);
        }

        if let Some(line) = self.lines.get_mut(self.cursor_pos.y) {
            for &ch in chars {
                if (self.cursor_pos.x as usize) < line.len() {
                    line[self.cursor_pos.x as usize].character = ch;
                    self.cursor_pos.x += 1;
                    if self.cursor_pos.x >= self.width {
                        self.new_line();
                        break;
                    }
                }
            }
        }
    }

    /// A helper function to check whether the terminal's screen should be rendered.
    pub fn should_render_now(&self) -> bool {
        use tokio::time::Instant;

        if !self.needs_render {
            return false;
        }

        let now = Instant::now();
        match self.last_render {
            Some(last) => now.duration_since(last) >= MIN_RENDER_INTERVAL,
            None => true,
        }
    }

    fn set_char_at_cursor(&mut self, ch: char) {
        while self.cursor_pos.y >= self.lines.len() {
            self.lines
                .push_back(vec![Cell::default(); self.width as usize]);
        }

        if let Some(line) = self.lines.get_mut(self.cursor_pos.y)
            && (self.cursor_pos.x as usize) < line.len()
        {
            line[self.cursor_pos.x as usize].character = ch;
        }
    }

    #[allow(clippy::needless_range_loop)]
    fn clear_from_cursor_to_eol(&mut self) {
        if let Some(line) = self.lines.get_mut(self.cursor_pos.y) {
            for x in (self.cursor_pos.x as usize)..line.len() {
                line[x] = Cell::default();
            }
        }
    }

    fn new_line(&mut self) {
        self.set_cursor_pos((0, self.cursor_pos.y + 1));

        if self.cursor_pos.y >= self.lines.len() {
            self.lines
                .push_back(vec![Cell::default(); self.width as usize]);
        }

        // Remove old lines if exceeding `ScreenBuffer.max_scrollback`
        while self.lines.len() > self.max_scrollback {
            self.lines.pop_front();
            // Update the view position
            if self.cursor_pos.y > 0 {
                self.cursor_pos.y -= 1;
            }
            if self.view_start > 0 {
                self.view_start -= 1;
            }
        }
    }

    /// Called to scroll the terminal up by `lines`.
    pub fn scroll_up(&mut self, lines: usize) {
        if self.view_start >= lines {
            self.view_start -= lines;
        } else {
            self.view_start = 0;
        }
        self.clear_selection();
        self.needs_render = true;
    }

    /// Called to scroll the terminal down by `lines`.
    pub fn scroll_down(&mut self, lines: usize) {
        let max_view_start = self.lines.len().saturating_sub(self.height as usize);
        self.view_start = (self.view_start + lines).min(max_view_start);
        self.clear_selection();
        self.needs_render = true;
    }

    /// Scrolls to the bottom of the screen. The bottom of the screen is
    /// the same as the most recent lines received from the serial connection
    pub fn scroll_to_bottom(&mut self) {
        self.view_start = self.lines.len().saturating_sub(self.height as usize);
        self.needs_render = true;
    }

    /// Scrolls to the top of the serial connection's history.
    pub fn scroll_to_top(&mut self) {
        self.view_start = 0;
        self.needs_render = true;
    }

    /// Sets the position within the screen for the start of a selection.
    /// Where `screen_x` is the x-position of the start of the selection,
    /// and `screen_y` is the y-position (line) of the start of the selection.
    pub fn start_selection(&mut self, screen_x: u16, screen_y: u16) {
        let absolute_line = self.view_start + screen_y as usize;
        self.clear_selection();
        self.selection_start = Some((screen_x, absolute_line));
        self.needs_render = true;
    }

    /// Update's a selection to include the position passed to it.
    /// Where `screen_x` is the x-position and `screen_y` is the y-position (line).
    pub fn update_selection(&mut self, screen_x: u16, screen_y: u16) {
        let absolute_line = self.view_start + screen_y as usize;
        self.selection_end = Some((screen_x, absolute_line));
        self.update_selection_highlighting();
        self.needs_render = true;
    }

    /// Clears the selection state.
    pub fn clear_selection(&mut self) {
        for line in &mut self.lines {
            for cell in line {
                cell.is_selected = false;
            }
        }
        self.selection_start = None;
        self.selection_end = None;
        self.needs_render = true;
    }

    fn update_selection_highlighting(&mut self) {
        for line in &mut self.lines {
            for cell in line {
                cell.is_selected = false;
            }
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
                        if let Some(cell) = line.get_mut(x as usize) {
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
                        if let Some(cell) = line.get(x as usize) {
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

    /// Copy's the currently selected text to the user's clipboard.
    pub fn copy_to_clipboard(&mut self) -> std::io::Result<()> {
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

    #[allow(dead_code)]
    fn get_stats(&self) -> BufferStats {
        let total_lines = self.lines.len();
        BufferStats {
            total_lines,
            view_start: self.view_start,
            view_end: (self.view_start + self.height as usize).min(total_lines),
            cursor_line: self.cursor_pos.y,
            has_selection: self.selection_start.is_some() && self.selection_end.is_some(),
        }
    }

    /// Clears the entire serial connection's history and reset's the screen.
    /// Similar to <kbd>Ctrl</kbd> + <kbd>l</kbd> in a terminal, except this will reset the
    /// connection's message history (on the user's side).
    pub fn clear_buffer(&mut self) {
        self.lines.clear();
        self.view_start = 0;
        self.set_cursor_pos((0_u16, 0_usize));
        self.lines
            .push_back(vec![Cell::default(); self.width as usize]);
        self.needs_render = true;
    }

    /// Writes the lines/characters received from `add_data` to the terminal's screen.
    /// As of now, `render` does not involve any diff-ing of previous renders.
    ///
    /// The nature of communicating to devices over a serial connection is similar
    /// that of a terminal; lines get printed to a screen and with each new line,
    /// all of the previously rendered characters must be re-rendered one cell higher.
    ///
    /// Because of this, the only diff-ing that would make sense would be
    /// that of the cells within the screen that are simply blank.
    pub fn render(&mut self) -> std::io::Result<()> {
        use crossterm::{cursor, queue, style};
        use std::io::{self, Write};
        use tokio::time::Instant;

        if !self.needs_render {
            return Ok(());
        }

        let mut writer = BufWriter::new(io::stdout());
        queue!(writer, cursor::Hide)?;

        for screen_y in 0..self.height {
            let line_idx = self.view_start + screen_y as usize;
            queue!(writer, cursor::MoveTo(0, screen_y))?;

            if let Some(line) = self.lines.get(line_idx) {
                let config = get_config();
                let mut current_fg = Color::from(&config.appearance.fg);
                let mut current_bg = Color::from(&config.appearance.bg);
                queue!(writer, style::SetForegroundColor(current_fg))?;
                queue!(writer, style::SetBackgroundColor(current_bg))?;

                for cell in line {
                    let fg = if cell.is_selected {
                        Color::from(&config.appearance.hl_fg)
                    } else {
                        cell.fg_color
                    };
                    let bg = if cell.is_selected {
                        Color::from(&config.appearance.hl_bg)
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
                    queue!(writer, style::Print(cell.character))?;
                }
            } else {
                queue!(writer, style::ResetColor)?;
                queue!(writer, style::Print(" ".repeat(self.width as usize)))?;
            }
        }

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

#[allow(dead_code)]
#[derive(Debug)]
struct BufferStats {
    pub total_lines: usize,
    pub view_start: usize,
    pub view_end: usize,
    pub cursor_line: usize,
    pub has_selection: bool,
}
