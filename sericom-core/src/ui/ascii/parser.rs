#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ParserEvent {
    Text(Vec<u8>),
    Control(u8),
    EscapeSequence(Vec<u8>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ParseState {
    Normal,
    Esc,
    Csi,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ByteParser {
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
                    0x1B => {
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
