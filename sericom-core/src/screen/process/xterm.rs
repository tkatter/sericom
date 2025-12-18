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

pub const NUL: u8 = 0x00; // Null character
pub const SOH: u8 = 0x01; // Start of Heading
pub const STX: u8 = 0x02; // Start of Text
pub const ETX: u8 = 0x03; // End of Text
pub const EOT: u8 = 0x04; // End of Transmission
pub const ENQ: u8 = 0x05; // Enquiry
pub const ACK: u8 = 0x06; // Acknowledge
pub const BEL: u8 = 0x07; // Bell, Alert
pub const BS: u8 = 0x08; // Backspace
pub const HT: u8 = 0x09; // Horizontal Tab
pub const NL: u8 = 0x0A; // Newline \n (Line Feed)
pub const VT: u8 = 0x0B; // Vertical Tabulation
pub const FF: u8 = 0x0C; // Form Feed
pub const CR: u8 = 0x0D; // Carriage Return
pub const SO: u8 = 0x0E; // Shift Out
pub const SI: u8 = 0x0F; // Shift In
pub const DLE: u8 = 0x10; // Data Link Escape
pub const DC1: u8 = 0x11; // Device Control One (XON)
pub const DC2: u8 = 0x12; // Device Control Two
pub const DC3: u8 = 0x13; // Device Control Three (XOFF)
pub const DC4: u8 = 0x14; // Device Control Four
pub const NAK: u8 = 0x15; // Negative Acknowledge
pub const SYN: u8 = 0x16; // Synchronous Idle
pub const ETB: u8 = 0x17; // End of Transmission Block
pub const CAN: u8 = 0x18; // Cancel
pub const EM: u8 = 0x19; // End of medium
pub const SUB: u8 = 0x1A; // Substitute
pub const ESC: u8 = 0x1B; // Escape
pub const FS: u8 = 0x1C; // File Separator
pub const GS: u8 = 0x1D; // Group Separator
pub const RS: u8 = 0x1E; // Record Separator
pub const US: u8 = 0x1F; // Unit Separator
pub const DEL: u8 = 0x7F; // Delete
pub const SEMI: u8 = b';'; // Escape sequence separator ';'

/// Reset graphics mode escape sequence
pub const RESET: &[u8] = &[ESC, b'[', b'0', b'm'];

/// Impl TryFrom for zig-like enum
macro_rules! def_xterm_tf {
    (
        #[repr($as:ty)]
        $(#[$attr:meta])*
        pub enum $i:ident {
            $(
                $(#[$doc:meta])?
                $variant:ident = $val:literal
            ),+
        }
        error = $err:literal
    ) => {
        #[repr($as)]
        $(#[$attr])*
        pub enum $i {
            $(
                $(#[$doc])?
                $variant = $val,
            )*
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
    #[doc = "C1 Control Codes (ESC _<C1>_)"]
    pub enum C1 {
        /// Index (0x84)
        IND = b'D',
        /// Next Line (0x85)
        NEL = b'E',
        /// Tab Set (0x88)
        HTS = b'H',
        /// Reverse Index (0x8d)
        RI = b'M',
        /// Device Control String (0x90)
        DCS = b'P',
        /// Start of Guarded Area (0x96)
        SPA = b'V',
        /// End of Guarded Area (0x97)
        EPA = b'W',
        /// Start of String (0x98)
        SOS = b'X',
        /// Return Terminal ID (0x9a)
        DECID = b'Z',
        /// Control Sequence Intoducer (0x9b)
        CSI = b'[',
        /// String Terminator (0x9c)
        ST = b'\\',
        /// Operating System Command (0x9d)
        OSC = b']',
        /// Privacy Message (0x9e)
        PM = b'^',
        /// Application Program Command (0x9f)
        APC = b'_',
        /// Back Index (VT420+)
        DECBI = b'6',
        /// Save Cursor (VT100)
        DECSC = b'7',
        /// Restore Cursor (VT100)
        DECRC = b'8',
        /// Forward Index (VT420+)
        DECFI = b'9',
        /// Full Reset (VT100)
        RIS = b'c'
    }
    error = "Could not parse {:00x?} as C1"
}

def_xterm_tf! {
    #[repr(u8)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum Cursor {
        CharRel = b'a',
        Up = b'A',
        Rep = b'b',
        Down = b'B',
        Forward = b'C',
        LineAbs = b'd',
        Backward = b'D',
        LineRel = b'e',
        NextLine = b'E',
        PositionHVP = b'f',
        PrecedingLine = b'F',
        CharAbsCHA = b'G',
        PositionCUP = b'H',
        ForwardTab = b'I',
        ScrollUp = b'S',
        ScrollDown = b'T',
        BackTab = b'Z',
        CharAbsHPA = b'`'
    }
    error = "Could not parse {:00x?} as Cursor"
}

def_xterm_tf! {
    #[repr(u8)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum EraseKind {
        Screen = b'J',
        Line = b'K',
        Chars = b'X'
    }
    error = "Could not parse {:00x?} as EraseKind"
}

def_xterm_tf! {
    #[repr(u8)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum EraseMode {
        Below = b'0',
        Above = b'1',
        All = b'2',
        Scrollback = b'3'
    }
    error = "Could not parse {:00x?} as EraseMode"
}

/* Where to stick these
* CSI Ps L  Insert Ps Line(s) (default = 1) (IL).
* CSI Ps M  Delete Ps Line(s) (default = 1) (DL).
* CSI Ps P  Delete Ps Character(s) (default = 1) (DCH).
*/
