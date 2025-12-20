//! Resources:
//! - [xterm's docs] The source
//! - [nwm article] Nice writeup, good format for quick review
//! - [xterm.js] Nice implementation/action descriptions
//! - [ascii table] Useful hex/binary/decimal ascii resource
//!
//! [ascii table]: https://www.ascii-code.com/
//! [xterm.js]: https://xtermjs.org/docs/api/vtfeatures/
//! [nwm article]: https://nicholas-morris.com/articles/ansi-codes
//! [xterm's docs]: https://invisible-island.net/xterm/ctlseqs/ctlseqs.html
#![allow(clippy::upper_case_acronyms)]

/// Reset graphics mode escape sequence
pub const RESET: &[u8] = &[0x1B, b'[', b'0', b'm'];
/// Escape sequence separator ';'
pub const SEMI: u8 = b';';

/// Impl TryFrom for zig-like enum
macro_rules! def_xterm_tf {
    (
        #[repr($as:ty)]
        $(#[$attr:meta])*
        pub enum $i:ident {
            $(
                $(#[$doc:meta])*
                $variant:ident = $val:literal
            ),* $(,)?
        }
        error = $err:literal
    ) => {
        #[repr($as)]
        $(#[$attr])*
        pub enum $i {
            $(
                $(#[$doc])*
                $variant = $val
            ),*
        }

        impl TryFrom<$as> for $i {
            type Error = $crate::SeriError;

            fn try_from(val: $as) -> Result<Self, Self::Error> {
                match val {
                    $($val => Ok($i::$variant),)+
                    _ => Err($crate::SeriError::Parsing(format!($err, val)))
                }
            }
        }
    };
}

def_xterm_tf! {
    #[repr(u8)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum C0 {
        /// Null character
        NUL = 0x00,
        /// Start of Heading
        SOH = 0x01,
        /// Start of Text
        STX = 0x02,
        /// End of Text
        ETX = 0x03,
        /// End of Transmission
        EOT = 0x04,
        /// Enquiry
        ENQ = 0x05,
        /// Acknowledge
        ACK = 0x06,
        /// Bell, Alert
        BEL = 0x07,
        /// Backspace
        BS = 0x08,
        /// Horizontal Tab
        HT = 0x09,
        /// Newline \n (Line Feed)
        NL = 0x0A,
        /// Vertical Tabulation
        VT = 0x0B,
        /// Form Feed
        FF = 0x0C,
        /// Carriage Return
        CR = 0x0D,
        /// Shift Out
        SO = 0x0E,
        /// Shift In
        SI = 0x0F,
        /// Data Link Escape
        DLE = 0x10,
        /// Device Control One (XON)
        DC1 = 0x11,
        /// Device Control Two
        DC2 = 0x12,
        /// Device Control Three (XOFF)
        DC3 = 0x13,
        /// Device Control Four
        DC4 = 0x14,
        /// Negative Acknowledge
        NAK = 0x15,
        /// Synchronous Idle
        SYN = 0x16,
        /// End of Transmission Block
        ETB = 0x17,
        /// Cancel
        CAN = 0x18,
        /// End of medium
        EM = 0x19,
        /// Substitute
        SUB = 0x1A,
        /// Escape
        ESC = 0x1B,
        /// File Separator
        FS = 0x1C,
        /// Group Separator
        GS = 0x1D,
        /// Record Separator
        RS = 0x1E,
        /// Unit Separator
        US = 0x1F,
        /// Delete
        DEL = 0x7F
    }
    error = "Could not parse {:#X?} as C0"
}

impl std::fmt::Display for C0 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:#X?}", *self as u8)
    }
}

def_xterm_tf! {
    #[repr(u8)]
    #[non_exhaustive]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    #[doc = "C1 Control Codes (ESC _<C1>_)"]
    pub enum C1 {
        /// Index 'D' (0x84)
        IND = b'D',
        /// Next Line 'E' (0x85)
        NEL = b'E',
        /// Tab Set 'H' (0x88)
        HTS = b'H',
        /// Reverse Index 'M' (0x8d)
        RI = b'M',
        /// Device Control String 'P' (0x90)
        DCS = b'P',
        /// Start of Guarded Area 'V' (0x96)
        SPA = b'V',
        /// End of Guarded Area 'W' (0x97)
        EPA = b'W',
        /// Start of String 'X' (0x98)
        SOS = b'X',
        /// Return Terminal ID 'Z' (0x9a)
        DECID = b'Z',
        /// Control Sequence Intoducer '[' (0x9b)
        CSI = b'[',
        /// String Terminator '\' (0x9c)
        ST = b'\\',
        /// Operating System Command ']' (0x9d)
        OSC = b']',
        /// Privacy Message '^' (0x9e)
        PM = b'^',
        /// Application Program Command '_' (0x9f)
        APC = b'_',
        /// Back Index '6' (VT420+)
        DECBI = b'6',
        /// Save Cursor '7' (VT100)
        DECSC = b'7',
        /// Restore Cursor '8' (VT100)
        DECRC = b'8',
        /// Forward Index '9' (VT420+)
        DECFI = b'9',
        /// Full Reset 'c' (VT100)
        RIS = b'c'
    }
    error = "Could not parse {:#X?} as C1"
}

impl std::fmt::Display for C1 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", *self as u8 as char)
    }
}

def_xterm_tf! {
    #[repr(u8)]
    #[non_exhaustive]
    #[allow(clippy::doc_markdown)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum CsiKind {
        /// Cursor Up _Ps_ Times 'A' (default = 1) (CUU)
        CursorUp = b'A',
        /// Cursor Down _Ps_ Times 'B' (default = 1) (CUD)
        CursorDown = b'B',
        /// Cursor Forward _Ps_ Times 'C' (default = 1) (CUF)
        CursorForward = b'C',
        /// Cursor Backward _Ps_ Times 'D' (default = 1) (CUB)
        CursorBackward = b'D',
        /// Cursor Next Line _Ps_ Times 'E' (default = 1) (CNL)
        NextLine = b'E',
        /// Cursor Preceding Line _Ps_ Times 'F' (default = 1) (CPL)
        PrecedingLine = b'F',
        /// Cursor Character Absolute [column] 'G' (default = [row,1]) (CHA)
        CharAbsCHA = b'G',
        /// Cursor Position [row;column] 'H' (default = [1,1]) (CUP)
        PositionCUP = b'H',
        /// Cursor Forward Tabulation _Ps_ tab stops 'I' (default = 1) (CHT)
        ForwardTab = b'I',
        /// Erase in Display 'J'
        ///
        /// VT100: CSI _Ps_ J (ED)
        /// VT220: CSI ? _Ps_ J (DECSED)
        EraseInDisplay = b'J',
        /// Erase in Line 'K'
        ///
        /// VT100: CSI _Ps_ K (EL)
        /// VT220: CSI ? _Ps_ K (DECSEL)
        EraseInLine = b'K',
        /// Insert _Ps_ Line(s) 'L' (default = 1) (IL)
        InsertLine = b'L',
        /// Delete _Ps_ Line(s) 'M' (default = 1) (DL)
        DeleteLine = b'M',
        /// Delete _Ps_ Character(s) 'P' (default = 1) (DCH)
        DeleteChars = b'P',
        /// Scroll up _Ps_ lines 'S' (default = 1) (SU)
        ScrollUp = b'S',
        /// Scroll down _Ps_ lines 'T' (default = 1) (SD)
        ScrollDown = b'T',
        /// Erase _Ps_ Character(s) 'X' (default = 1) (ECH)
        EraseChars = b'X',
        /// Cursor Backward Tabulation _Ps_ tab stops 'Z' (default = 1) (CBT)
        BackTab = b'Z',
        /// Character Position Relative [columns] 'a' (default = [row,col+1]) (HPR)
        CharRel = b'a',
        /// Repeat the preceding graphic character _Ps_ times 'b' (REP)
        Rep = b'b',
        /// Line Position Absolute [row] 'd' (default = [1,column]) (VPA)
        LineAbs = b'd',
        /// Line Position Relative  [rows] 'e' (default = [row+1,column]) (VPR)
        LineRel = b'e',
        /// Horizontal and Vertical Position [row;column] 'f' (default = [1,1]) (HVP)
        PositionHVP = b'f',
        /// Character Attributes 'm' (SGR)
        SGR = b'm',
        /// Character Position Absolute [column] '`' (default = [row,1]) (HPA)
        CharAbsHPA = b'`',
        /// Scroll down _Ps_ lines '^' (default = 1) (SD)
        ///
        /// From [xterm]:
        /// > This was a publication error in the original ECMA-48 5th edition
        /// > (1991) corrected in 2003.
        ///
        /// [xterm]: https://invisible-island.net/xterm/ctlseqs/ctlseqs.html#h3-Functions-using-CSI-_-ordered-by-the-final-character_s_
        ScrollDown1991 = b'^'
    }
    error = "Could not parse {:#X?} as CsiKind"
}

impl std::fmt::Display for CsiKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", *self as u8 as char)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CSI<'a> {
    pub kind: CsiKind,
    pub params: &'a [u8],
}

impl std::fmt::Display for CSI<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ESC [ {} {}",
            // SAFETY: It is verified that self.params is ascii when constructed
            // in ByteParser via core::num::is_ascii()
            unsafe { std::str::from_utf8_unchecked(self.params) },
            self.kind
        )
    }
}

def_xterm_tf! {
    #[repr(u8)]
    #[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
    #[doc = "Erase in Display (ED) kinds"]
    pub enum EDKind {
        #[default]
        Below = b'0',
        Above = b'1',
        All = b'2',
        Saved = b'3'
    }
    error = "Could not parse {:#X?} as EDKind"
}

def_xterm_tf! {
    #[repr(u8)]
    #[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
    #[doc = "Erase in Line (EL) kinds"]
    pub enum ELKind {
        #[default]
        Right = b'0',
        Left = b'1',
        All = b'2',
    }
    error = "Could not parse {:#X?} as ELKind"
}

impl TryFrom<&[u8]> for EDKind {
    type Error = crate::SeriError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        if value.is_empty() {
            Err(crate::SeriError::Parsing(
                "Could not parse as EDKind".to_string(),
            ))
        } else {
            Self::try_from(value[0])
        }
    }
}

impl TryFrom<&[u8]> for ELKind {
    type Error = crate::SeriError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        if value.is_empty() {
            Err(crate::SeriError::Parsing(
                "Could not parse as ELKind".to_string(),
            ))
        } else {
            Self::try_from(value[0])
        }
    }
}
