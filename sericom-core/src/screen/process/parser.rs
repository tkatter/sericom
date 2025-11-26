use super::ESC;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParserEvent {
    Text(Vec<u8>),
    Control(u8),
    EscapeSequence(Vec<u8>),
}

impl std::fmt::Display for ParserEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use crate::screen::driver::EscSequenceType;
        use crate::screen::driver::classify_escape_seq;
        match self {
            Self::Text(items) => {
                let s = str::from_utf8(items).expect("parsed text is utf-8");
                f.write_fmt(format_args!("Text( {s} )"))
            }
            Self::Control(b) => f.write_fmt(format_args!("Control( {b:#X} )")),
            Self::EscapeSequence(items) => {
                let Some(etype) = classify_escape_seq(items) else {
                    return f.write_str("INVALID ESC SEQ");
                };

                let s = match etype {
                    EscSequenceType::Cursor(_) => {
                        let mut s = String::new();
                        for b in &items[2..items.len()] {
                            s.push(char::from(*b));
                        }
                        format!("Cursor( ESC[{s} )")
                    }
                    EscSequenceType::Erase(_) => {
                        let mut s = String::new();
                        for b in &items[2..items.len()] {
                            s.push(char::from(*b));
                        }
                        format!("Erase( ESC[{s} )")
                    }
                    EscSequenceType::Graphics => {
                        let mut s = String::new();
                        for b in &items[2..items.len()] {
                            s.push(char::from(*b));
                        }
                        format!("Graphics( ESC[{s} )")
                    }
                    EscSequenceType::Screen(_) => {
                        let mut s = String::new();
                        for b in &items[2..items.len()] {
                            s.push(char::from(*b));
                        }
                        format!("Screen( ESC[{s} )")
                    }
                };

                f.write_str(&s)
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseState {
    Normal,
    Esc,
    Csi,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ByteParser {
    state: ParseState,
    buffer: Vec<u8>,
}

impl ByteParser {
    pub(crate) const fn new() -> Self {
        Self {
            state: ParseState::Normal,
            buffer: vec![],
        }
    }
}

impl ByteParser {
    pub(crate) fn feed(&mut self, data: &[u8]) -> Vec<ParserEvent> {
        let mut events: Vec<ParserEvent> = Vec::new();

        for &b in data {
            if !b.is_ascii() {
                continue;
            }
            match self.state {
                ParseState::Normal => match b {
                    ESC => {
                        if !self.buffer.is_empty() {
                            events.push(ParserEvent::Text(std::mem::take(&mut self.buffer)));
                        }
                        self.buffer.push(b);
                        self.state = ParseState::Esc;
                    }
                    0x00..=0x1F | 0x7F => {
                        if !self.buffer.is_empty() {
                            events.push(ParserEvent::Text(std::mem::take(&mut self.buffer)));
                        }
                        events.push(ParserEvent::Control(b));
                    }
                    // Regular text
                    _ => {
                        self.buffer.push(b);
                    }
                },
                ParseState::Esc => {
                    self.buffer.push(b);
                    if b == b'[' {
                        self.state = ParseState::Csi;
                    } else {
                        // Incomplete escape sequence but push as escape sequence anyway if
                        // it is nonesense - the consumer will not do anything with it later.
                        events.push(ParserEvent::EscapeSequence(std::mem::take(
                            &mut self.buffer,
                        )));
                        self.state = ParseState::Normal;
                    }
                }
                ParseState::Csi => {
                    self.buffer.push(b);
                    // Csi is terminated by a regular letter [a-z][A-Z]
                    if b.is_ascii_alphabetic() {
                        events.push(ParserEvent::EscapeSequence(std::mem::take(
                            &mut self.buffer,
                        )));
                        self.state = ParseState::Normal;
                    }
                }
            }
        }

        // Flush buffer if it is regular text
        if self.state == ParseState::Normal && !self.buffer.is_empty() {
            events.push(ParserEvent::Text(std::mem::take(&mut self.buffer)));
        }
        events
    }
}
