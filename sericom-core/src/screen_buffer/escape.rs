use tracing::debug;

use super::{Cursor, Line, Position, ScreenBuffer};
use crate::screen_buffer::UIAction;

/// `EscapeState` holds stateful information about the incoming
/// data to allow for proper processing of ansii escape codes/characters.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum EscapeState {
    /// Has not received ansii escape characters
    Normal,
    /// Just received an ESC (0x1B)
    Esc,
    /// Received ESC and then '[' (0x5B)
    Csi,
}

/// Represents a section of an ascii escape sequence.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum EscapePart {
    /// The default state when not actively processing an escape sequence.
    Empty,
    /// Collects ascii digits (0-9) as they are received individually and are
    /// eventually combined to create the final number that the `Action` will
    /// perform on i.e. `vec!['2', '3']` -> `23`.
    Numbers(Vec<char>),
    /// The `Separator` represents the `;` used in ascii escape sequences.
    Separator,
    /// The `Action` represents the (typically) last letter of an escape
    /// sequence that determines what action is to be taken i.e. `ESC[2J`.
    Action(char),
}

impl Default for EscapePart {
    fn default() -> Self {
        Self::Empty
    }
}

/// A state-holder/collection for building ascii escape sequences
/// from incoming data to enable proper processing/execution.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct EscapeSequence {
    sequence: Vec<EscapePart>,
    part: EscapePart,
}

impl EscapeSequence {
    pub(super) fn new() -> Self {
        Self {
            sequence: Vec::new(),
            part: EscapePart::Empty,
        }
    }

    /// Clear's the sequence and sets the part to [`EscapePart::Empty`].
    pub(super) fn reset(&mut self) {
        // Clear is probably good since it will continue to
        // fill up to similar sizes throughout the program.
        self.sequence.clear();
        self.part = EscapePart::Empty;
    }

    /// Appends the `part` that is currently being processed to the `sequence`.
    fn push_part(&mut self) {
        self.sequence.push(std::mem::take(&mut self.part));
    }

    /// Appends a [`EscapePart::Separator`] to `Self.sequence`.
    pub(super) fn insert_separator(&mut self) {
        if self.part != EscapePart::Empty {
            self.push_part();
        }
        self.sequence.push(EscapePart::Separator);
    }

    /// Adds numbers to the in-progress part of the escape sequence
    pub(super) fn push_num(&mut self, num: char) {
        match &mut self.part {
            EscapePart::Numbers(nums) => nums.push(num),
            _ => self.part = EscapePart::Numbers(vec![num]),
        }
    }

    /// Pushes the action to the escape sequence, signaling the end
    /// and results in carrying out the action for the escape sequence
    /// and then resetting its values.
    pub(super) fn push_action(&mut self, action: char) {
        if self.part != EscapePart::Empty {
            self.push_part();
        }
        self.sequence.push(EscapePart::Action(action));
    }
}

impl ScreenBuffer {
    pub(crate) fn parse_sequence(&mut self) {
        let span = tracing::span!(tracing::Level::DEBUG, "Escape sequence");
        let _enter = span.enter();
        match &self.escape_sequence.sequence[..] {
            [
                EscapePart::Numbers(line_nums),
                EscapePart::Separator,
                EscapePart::Numbers(col_nums),
                EscapePart::Action(action),
            ] => {
                debug!("Got 'ESC[{:?};{:?}{}'", line_nums, col_nums, action);
                match action {
                    // Move cursor to (line_num, col_num)
                    'H' | 'f' => {
                        // Can unwrap because it is guaranteed elsewhere that
                        // `EscapePart::Numbers(Vec<Char>)` only holds ascii digits (0-9).
                        let mut line_num: u16 =
                            line_nums.iter().collect::<String>().parse().unwrap();
                        let col_num: u16 = col_nums.iter().collect::<String>().parse().unwrap();
                        if line_num <= 1 {
                            line_num = self.lines.len() as u16 - self.height;
                        } else {
                            line_num += self.lines.len() as u16 - self.height;
                        }
                        self.set_cursor_pos((col_num, line_num));
                    }
                    _ => {}
                }
                self.escape_state = EscapeState::Normal;
            }
            [
                EscapePart::Separator,
                EscapePart::Numbers(col_nums),
                EscapePart::Action(action),
            ] => {
                debug!("Got 'ESC[;{:?}{}'", col_nums, action);
                match action {
                    // Move cursor to (same, col_num)
                    'H' | 'f' => {
                        // Can unwrap because it is guaranteed elsewhere that
                        // `EscapePart::Numbers(Vec<Char>)` only holds ascii digits (0-9).
                        let col_num: u16 = col_nums.iter().collect::<String>().parse().unwrap();
                        self.cursor_pos.x = col_num;
                    }
                    _ => {}
                }
                self.escape_state = EscapeState::Normal;
            }
            [EscapePart::Numbers(nums), EscapePart::Action(action)] => {
                debug!("Got 'ESC[{:?}{}'", nums, action);
                // Can unwrap because it is guaranteed elsewhere that
                // `EscapePart::Numbers(Vec<Char>)` only holds ascii digits (0-9).
                let num: u16 = nums.iter().collect::<String>().parse().unwrap();
                // NOTE: These functions are solely doing what they say and do NOT move the cursor
                match (num, action) {
                    // Move cursor up # of lines
                    (num, 'A') => self.move_cursor_up(num),
                    // Move cursor down # of lines
                    (num, 'B') => self.move_cursor_down(num),
                    // Move cursor right # of cols
                    (num, 'C') => self.move_cursor_right(num),
                    // Move cursor left # of cols
                    (num, 'D') => self.move_cursor_left(num),
                    // Moves cursor to beginning of line, # lines down
                    (num, 'E') => {
                        self.set_cursor_pos((0, (self.cursor_pos.y as u16) + num));
                        while self.cursor_pos.y > self.lines.len() {
                            self.lines.push_back(Line::new(self.width as usize));
                        }
                    }
                    // Moves cursor to beginning of line, # lines up
                    (num, 'F') => self.set_cursor_pos((0, (self.cursor_pos.y as u16) - num)),
                    // Moves cursor to column #
                    (num, 'G') => self.set_cursor_col(num),
                    // Erase from cursor until end of screen
                    (0, 'J') => self.clear_from_cursor_to_eos(),
                    // Erase from cursor to beginning of screen
                    (1, 'J') => self.clear_from_cursor_to_sos(),
                    // Erase entire screen
                    (2, 'J') => self.clear_screen(),
                    // Erase from cursor to end of line
                    (0, 'K') => self.clear_from_cursor_to_eol(),
                    // Erase start of line to cursor
                    (1, 'K') => self.clear_from_cursor_to_sol(),
                    // Erase entire line
                    (2, 'K') => self.clear_whole_line(),
                    _ => {}
                }
                self.escape_state = EscapeState::Normal;
            }
            [EscapePart::Action(action)] => {
                debug!("Got 'ESC[{}'", action);
                match action {
                    // Set cursor position to 0, 0
                    'H' => self.cursor_pos = Position::home(),
                    // Erase from cursor until end of screen
                    'J' => self.clear_from_cursor_to_eos(),
                    // Erase from cursor to end of line
                    'K' => self.clear_from_cursor_to_eol(),
                    'C' => self.move_cursor_right(1),
                    'D' => self.move_cursor_left(1),
                    action if action.is_alphabetic() => {}
                    _ => {}
                }
                self.escape_state = EscapeState::Normal;
            }
            _ => self.escape_state = EscapeState::Normal,
        }
    }
}
