use crate::screen::process::{C0, C1, CSI, CsiKind};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParserEvent<'a> {
    Text(&'a [u8]),
    CSI(CSI<'a>),
    C0(C0),
    C1(C1),
}

impl<'a> std::fmt::Display for ParserEvent<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Text(items) => {
                // SAFETY: Only bytes within the ascii printable text range
                // are processed as Self::Text
                write!(f, "Text( {} )", unsafe { str::from_utf8_unchecked(items) })
            }
            Self::C0(c0) => write!(f, "C0( {c0} )"),
            Self::C1(c1) => write!(f, "C1( {c1} )"),
            Self::CSI(csi) => write!(f, "CSI( {csi} )"),
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub enum ParseState {
    #[default]
    Normal,
    Esc,
    Csi,
    Osc,
    Dcs,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ByteParser {
    state: ParseState,
    start_idx: Option<usize>,
}

impl ByteParser {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            state: ParseState::Normal,
            start_idx: None,
        }
    }

    pub fn feed<'a>(&mut self, data: &'a [u8]) -> Vec<ParserEvent<'a>> {
        let mut events: Vec<ParserEvent> = Vec::new();
        for (idx, &b) in data.iter().enumerate() {
            if !b.is_ascii() {
                continue;
            }

            match self.state {
                ParseState::Normal => match b {
                    0x00..=0x1F | 0x7F => {
                        if let Ok(ctrl) = C0::try_from(b) {
                            if let Some(start) = self.start_idx.take() {
                                events.push(ParserEvent::Text(&data[start..idx]));
                            }

                            if ctrl == C0::ESC {
                                self.state = ParseState::Esc;
                            } else {
                                events.push(ParserEvent::C0(ctrl));
                            }
                        }
                    }
                    // Regular text
                    _ if self.start_idx.is_none() => {
                        self.start_idx = Some(idx);
                    }
                    _ => {}
                },
                ParseState::Esc => {
                    let Ok(ctrl) = C1::try_from(b) else {
                        self.state = ParseState::Normal;
                        continue;
                    };
                    match ctrl {
                        C1::CSI => self.state = ParseState::Csi,
                        C1::DCS => self.state = ParseState::Dcs,
                        C1::OSC => self.state = ParseState::Osc,
                        _ => {
                            events.push(ParserEvent::C1(ctrl));
                            self.state = ParseState::Normal;
                        }
                    }
                }
                ParseState::Csi => {
                    if let Ok(kind) = CsiKind::try_from(b) {
                        match self.start_idx.take() {
                            Some(start) => {
                                events.push(ParserEvent::CSI(CSI {
                                    kind,
                                    params: &data[start..idx],
                                }));
                            }
                            None => events.push(ParserEvent::CSI(CSI { kind, params: &[] })),
                        }
                        self.state = ParseState::Normal;
                    } else {
                        self.start_idx = Some(idx);
                    }
                }
                ParseState::Dcs => todo!(),
                ParseState::Osc => todo!(),
            }
        }

        // Flush buffer if it is regular text
        if self.state == ParseState::Normal
            && let Some(start) = self.start_idx.take()
        {
            events.push(ParserEvent::Text(&data[start..]));
        }
        events
    }
}
