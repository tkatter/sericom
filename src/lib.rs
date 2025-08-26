pub mod screen_buffer {
    use std::{collections::VecDeque, io::BufWriter};
    use crossterm::style::Color;

    const MIN_RENDER_INTERVAL: tokio::time::Duration = tokio::time::Duration::from_millis(33);

    #[derive(Clone, Debug)]
    pub enum UICommand {
        ScrollUp(usize),
        ScrollDown(usize),
        StartSelection(u16, u16),
        UpdateSelection(u16, u16),
        CopySelection,
        ClearSelection,
        ClearBuffer,
    }

    #[derive(Clone, Debug)]
    pub struct Cell {
        pub character: char,
        pub fg_color: Color,
        pub bg_color: Color,
        pub is_selected: bool,
    }

    impl Default for Cell {
        fn default() -> Self {
            Self {
                character: ' ',
                fg_color: Color::Green,
                bg_color: Color::Reset,
                is_selected: false,
            }
        }
    }

    /// The `ScreenBuffer` is used to keep track of different parts of the
    /// terminal's state throughout a serial connection's session.
    ///
    /// The state is used to allow for further functionality within the terminal's
    /// runtime such as scrolling, selecting text, and copying to a clipboard.
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
        /// Denotes the first *writeable* column for the current line.
        input_start_x: u16,
        cursor_x: u16,
        cursor_y: usize,
        selection_start: Option<(u16, usize)>,
        selection_end: Option<(u16, usize)>,
        /// Configuration for the maximum amount of lines to keep in memory.
        max_scrollback: usize,
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
                input_start_x: 0,
                cursor_x: 0,
                cursor_y: 0,
                selection_start: None,
                selection_end: None,
                max_scrollback,
                last_render: None,
                needs_render: false,
            };
            // Start with an empty line
            buffer.lines.push_back(vec![Cell::default(); width as usize]);
            buffer
        }

        pub fn add_data(&mut self, data: &[u8]) {
            let text = String::from_utf8_lossy(data);
            let mut chars = text.chars().peekable();

            while let Some(ch) = chars.next() {
                match ch {
                    '\r' => {
                        self.cursor_x = 0;
                        if chars.peek() == Some(&'\n') {
                            chars.next();
                            self.new_line();
                        }
                    }
                    '\n' => {
                        self.new_line();
                    }
                    '\x08' => {
                        if self.cursor_x > self.input_start_x {
                        // if self.cursor_x > 0 {
                            self.cursor_x -= 1;
                            // self.set_char_at_cursor(' ');
                        }
                    }
                    c => {
                        let mut batch = vec![c];
                        while let Some(&next_ch) = chars.peek() {
                            if next_ch.is_control() || self.cursor_x + batch.len() as u16 >= self.width {
                                break;
                            }
                            batch.push(chars.next().unwrap());
                        }
                        self.add_char_batch(&batch);
                    }
                }
            }
            self.scroll_to_bottom();
            self.needs_render = true;
        }

        fn add_char_batch(&mut self, chars: &[char]) {
            while self.cursor_y >= self.lines.len() {
                self.lines.push_back(vec![Cell::default(); self.width as usize]);
            }

            if let Some(line) = self.lines.get_mut(self.cursor_y) {
                for &ch in chars {
                    if (self.cursor_x as usize) < line.len() {
                        line[self.cursor_x as usize].character = ch;
                        self.cursor_x += 1;
                        if self.cursor_x >= self.width {
                            self.new_line();
                            break;
                        }
                    }
                }
            }
        }

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

        // fn set_char_at_cursor(&mut self, ch: char) {
        //     while self.cursor_y >= self.lines.len() {
        //         self.lines.push_back(vec![Cell::default(); self.width as usize]);
        //     }
        //
        //     if let Some(line) = self.lines.get_mut(self.cursor_y) {
        //         if (self.cursor_x as usize) < line.len() {
        //             line[self.cursor_x as usize].character = ch;
        //         }
        //     }
        // }

        fn new_line(&mut self) {
            self.cursor_y += 1;
            self.cursor_x = 0;

            if self.cursor_y >= self.lines.len() {
                self.lines.push_back(vec![Cell::default(); self.width as usize]);
            }

            self.input_start_x = 0;

            // Remove old lines if exceeding `ScreenBuffer.max_scrollback`
            while self.lines.len() > self.max_scrollback {
                self.lines.pop_front();
                // Update the view position
                if self.cursor_y > 0 {
                    self.cursor_y -= 1;
                }
                if self.view_start > 0 {
                    self.view_start -= 1;
                }
            }
        }

        pub fn scroll_up(&mut self, lines: usize) {
            if self.view_start >= lines {
                self.view_start -= lines;
            } else {
                self.view_start = 0;
            }
            self.clear_selection();
            self.needs_render = true;
        }

        pub fn scroll_down(&mut self, lines: usize) {
            let max_view_start = self.lines.len().saturating_sub(self.height as usize);
            self.view_start = (self.view_start + lines).min(max_view_start);
            self.clear_selection();
            self.needs_render = true;
        }

        pub fn scroll_to_bottom(&mut self) {
            self.view_start = self.lines.len().saturating_sub(self.height as usize);
            self.needs_render = true;
        }

        pub fn scroll_to_top(&mut self) {
            self.view_start = 0;
            self.needs_render = true;
        }

        pub fn start_selection(&mut self, screen_x: u16, screen_y: u16) {
            let absolute_line = self.view_start + screen_y as usize;
            self.clear_selection();
            self.selection_start = Some((screen_x, absolute_line));
            self.needs_render = true;
        }

        pub fn update_selection(&mut self, screen_x: u16, screen_y: u16) {
            let absolute_line = self.view_start + screen_y as usize;
            self.selection_end = Some((screen_x, absolute_line));
            self.update_selection_highlighting();
            self.needs_render = true;
        }

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
                (self.selection_start, self.selection_end) {
                    let (start_line, start_x, end_line, end_x) = if start_line < end_line ||
                        (start_line == end_line && start_x <= end_x) {
                            (start_line, start_x, end_line, end_x)
                    } else {
                        (end_line, end_x, start_line, start_x)
                    };

                    for line_idx in start_line..=end_line {
                        if let Some(line) = self.lines.get_mut(line_idx) {
                            let line_start_x = if line_idx == start_line { start_x } else { 0 };
                            let line_end_x = if line_idx == end_line { end_x } else { self.width - 1 };

                            for x in line_start_x..=line_end_x.min(self.width - 1) {
                                if let Some(cell) = line.get_mut(x as usize) {
                                    cell.is_selected = true;
                                }
                            }
                        }
                    }
            }
        }

        pub fn get_selected_text(&self) -> String {
            if let (Some((start_x, start_line)), Some((end_x, end_line))) =
                (self.selection_start, self.selection_end) {
                    let (start_line, start_x, end_line, end_x) = if start_line < end_line ||
                        (start_line == end_line && start_x <= end_x) {
                            (start_line, start_x, end_line, end_x)
                    } else {
                        (end_line, end_x, start_line, start_x)
                    };

                    let mut result = String::new();

                    for line_idx in start_line..=end_line {
                        if let Some(line) = self.lines.get(line_idx) {
                            let line_start_x = if line_idx == start_line { start_x } else { 0 };
                            let line_end_x = if line_idx == end_line { end_x } else { self.width - 1 };

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

        pub fn copy_to_clipboard(&mut self) -> std::io::Result<()> {
            use crossterm::{ clipboard, execute};

            let selected_text = self.get_selected_text();
            if !selected_text.is_empty() {
                execute!(std::io::stdout(), clipboard::CopyToClipboard::to_clipboard_from(selected_text))?;
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
                cursor_line: self.cursor_y,
                has_selection: self.selection_start.is_some() && self.selection_end.is_some(),
            }
        }

        pub fn clear_buffer(&mut self) {
            self.lines.clear();
            self.view_start = 0;
            self.cursor_x = 0;
            self.cursor_y = 0;
            self.lines.push_back(vec![Cell::default(); self.width as usize]);
            self.needs_render = true;
            self.input_start_x = 0;
        }

        pub fn set_input_start(&mut self, prompt_len: u16) {
            self.input_start_x = prompt_len;
            self.cursor_x = prompt_len;
            self.needs_render = true;
        }

        pub fn render(&mut self) -> std::io::Result<()> {
            use std::io::{self, Write};
            use crossterm::{ queue, cursor, style };
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
                    let mut current_fg = Color::Green;
                    let mut current_bg = Color::Reset;
                    queue!(writer, style::SetForegroundColor(current_fg))?;
                    queue!(writer, style::SetBackgroundColor(current_bg))?;

                    for cell in line {
                        let fg = if cell.is_selected { Color::Black } else { cell.fg_color };
                        let bg = if cell.is_selected { Color::White } else { cell.bg_color };
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

            let screen_cursor_y = if self.cursor_y >= self.view_start &&
                self.cursor_y < self.view_start + self.height as usize {
                    (self.cursor_y - self.view_start) as u16
            } else {
                self.height - 1
            };

            queue!(writer, cursor::MoveTo(self.cursor_x, screen_cursor_y), cursor::Show)?;
            writer.flush()?;
            self.last_render = Some(Instant::now());
            self.needs_render = false;
            Ok(())
        }
    }

    #[derive(Debug)]
    pub struct BufferStats {
        pub total_lines: usize,
        pub view_start: usize,
        pub view_end: usize,
        pub cursor_line: usize,
        pub has_selection: bool,
    }
}
