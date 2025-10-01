mod parser;
pub mod process;

#[cfg(test)]
mod test;

pub(crate) use parser::*;

/// Bracket '['
pub(crate) const BK: u8 = b'[';
/// Backspace
pub(crate) const BS: u8 = 0x08;
/// Carrige return '\r'
pub(crate) const CR: u8 = 0x0D;
/// Escape 'ESC'
pub(crate) const ESC: u8 = 0x1B;
/// Newline '\n'
pub(crate) const NL: u8 = 0x0A;
/// Escape sequence separator ';'
pub(crate) const SEP: u8 = b';';
/// Tab '\t'
pub(crate) const TAB: u8 = 0x09;
/// Form feed
pub(crate) const FF: u8 = 0x0C;
/// Reset graphics mode escape sequence
pub(crate) const RESET: &[u8] = &[ESC, BK, b'0', b'm'];
