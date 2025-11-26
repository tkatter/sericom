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
//! currently, the **capacity of the [`VecDeque`] is hardcoded with a value of 10,000
//! lines with [`MAX_SCROLLBACK`]**.
#![deny(dead_code)]
#![allow(unused)]

mod buffer;
mod components;
mod driver;
mod position;
pub mod process;
mod rect;
mod render;
mod ui_command;

pub use buffer::ScreenBuffer;
pub use components::{Cell, Line, Span};
pub use position::{BuffPos, Cursor, PosType, PosY, Position, Scope, TermPos, TranslatePos};
pub use process::{ByteParser, ColorState, ParseState, ParserEvent};
pub use rect::Rect;
pub use ui_command::{UIAction, UICommand};

#[cfg(test)]
pub(crate) mod tests;

pub(in crate::screen) use process::{
    process_colors, process_cursor, process_erase, process_screen,
};
