use serde::Deserialize;
use std::borrow::Cow;

/// Represents the `[appearance]` table of the `config.toml` file.
///
/// The `[appearance]` table holds configuration values for sericom's appearance.
///
/// The default values (if no config exists) is the current directory:
/// ```toml
/// [appearance]
/// fg = "green"
/// bg = "none"
/// hl_fg = "black"
/// hl_bg = "white"
/// ```
#[derive(Debug, Deserialize, PartialEq)]
pub struct Appearance {
    #[serde(default = "default_fg")]
    pub fg: SeriColor,
    #[serde(default = "default_bg")]
    pub bg: SeriColor,
    #[serde(default = "default_hl_fg")]
    pub hl_fg: SeriColor,
    #[serde(default = "default_hl_bg")]
    pub hl_bg: SeriColor,
}

fn default_hl_fg() -> SeriColor {
    SeriColor::Black
}

fn default_hl_bg() -> SeriColor {
    SeriColor::White
}

fn default_fg() -> SeriColor {
    SeriColor::Green
}
fn default_bg() -> SeriColor {
    SeriColor::None
}

impl Default for Appearance {
    fn default() -> Self {
        Self {
            fg: SeriColor::Green,
            bg: SeriColor::None,
            hl_fg: SeriColor::Black,
            hl_bg: SeriColor::White,
        }
    }
}

/// Uses [`Cow`] to normalize strings passed to it. If the input is already
/// normalized, it simply returns a [`Cow::Borrowed`], else will remove
/// '-', '_', and whitespace and transform to lowercase.
pub const NORMALIZER: fn(&str) -> Cow<'_, str> = normalizer;

fn normalizer(s: &str) -> Cow<'_, str> {
    let mut curr_cow: Cow<'_, str> = Cow::Borrowed(s);

    if curr_cow.contains(' ') || curr_cow.contains('-') || curr_cow.contains('_') {
        let owned_str = curr_cow.to_mut();
        *owned_str = owned_str.replace(['-', '_'], "");
    }

    if curr_cow.chars().any(|c| c.is_ascii_uppercase()) {
        let owned_str = curr_cow.to_mut();
        *owned_str = owned_str.to_lowercase();
    }
    curr_cow
}

/// Represents a list of the valid options for [`SeriColor`].
pub static VALID_SERICOLORS: &[&str] = &[
    "black",
    "blue",
    "cyan",
    "dark-blue",
    "dark-cyan",
    "dark-green",
    "dark-grey",
    "dark-magenta",
    "dark-red",
    "dark-yellow",
    "default",
    "green",
    "grey",
    "magenta",
    "red",
    "white",
    "yellow",
];

/// A wrapper around [`crossterm::style::Color`] to allow for implementing serde's
/// [`Deserialize`] beyond the default implementation from `#[derive(Deserialize)]`
#[derive(Debug, PartialEq, Clone, Eq)]
pub enum SeriColor {
    Black,
    Blue,
    Cyan,
    DarkBlue,
    DarkCyan,
    DarkGreen,
    DarkGrey,
    DarkMagenta,
    DarkRed,
    DarkYellow,
    Green,
    Grey,
    Magenta,
    None,
    Red,
    White,
    Yellow,
}

impl SeriColor {
    /// Parses `input` to a [`SeriColor`]. Returns `Ok(SeriColor)` if successful,
    /// otherwise returns `Err(VALID_SERICOLORS)`.
    ///
    /// ## Example
    ///
    /// ```
    /// use sericom_core::configs::*;
    ///
    /// fn get_color() {
    ///     let color = SeriColor::parse_from_str("Dark-Green", NORMALIZER);
    ///     assert_eq!(color, Ok(SeriColor::DarkGreen));
    /// }
    /// ```
    pub fn parse_from_str<S, F>(input: S, normalizer: F) -> Result<Self, &'static [&'static str]>
    where
        S: AsRef<str>,
        F: Fn(&str) -> Cow<'_, str>,
    {
        let input_slice = input.as_ref();
        let normalized_cow = normalizer(input_slice);
        let normalized_str = normalized_cow.as_ref();
        match normalized_str {
            "black" => Ok(SeriColor::Black),
            "blue" => Ok(SeriColor::Blue),
            "cyan" => Ok(SeriColor::Cyan),
            "darkblue" => Ok(SeriColor::DarkBlue),
            "darkcyan" => Ok(SeriColor::DarkCyan),
            "darkgreen" => Ok(SeriColor::DarkGreen),
            "darkgrey" | "darkgray" => Ok(SeriColor::DarkGrey),
            "darkmagenta" => Ok(SeriColor::DarkMagenta),
            "darkred" => Ok(SeriColor::DarkRed),
            "darkyellow" => Ok(SeriColor::DarkYellow),
            "default" => Ok(SeriColor::None),
            "green" => Ok(SeriColor::Green),
            "grey" | "gray" => Ok(SeriColor::Grey),
            "magenta" => Ok(SeriColor::Magenta),
            "red" => Ok(SeriColor::Red),
            "white" => Ok(SeriColor::White),
            "yellow" => Ok(SeriColor::Yellow),
            _ => Err(VALID_SERICOLORS),
        }
    }
}

impl<'de> Deserialize<'de> for SeriColor {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        match SeriColor::parse_from_str(&s, NORMALIZER) {
            Ok(s) => Ok(s),
            Err(valid_colors) => Err(serde::de::Error::unknown_variant(&s, valid_colors)),
        }
    }
}

impl From<&SeriColor> for crossterm::style::Color {
    fn from(value: &SeriColor) -> crossterm::style::Color {
        match value {
            SeriColor::Black => crossterm::style::Color::Black,
            SeriColor::Blue => crossterm::style::Color::Blue,
            SeriColor::Cyan => crossterm::style::Color::Cyan,
            SeriColor::DarkBlue => crossterm::style::Color::DarkBlue,
            SeriColor::DarkCyan => crossterm::style::Color::DarkCyan,
            SeriColor::DarkGreen => crossterm::style::Color::DarkGreen,
            SeriColor::DarkGrey => crossterm::style::Color::DarkGrey,
            SeriColor::DarkMagenta => crossterm::style::Color::DarkMagenta,
            SeriColor::DarkRed => crossterm::style::Color::DarkRed,
            SeriColor::DarkYellow => crossterm::style::Color::DarkYellow,
            SeriColor::Green => crossterm::style::Color::Green,
            SeriColor::Grey => crossterm::style::Color::Grey,
            SeriColor::Magenta => crossterm::style::Color::Magenta,
            SeriColor::None => crossterm::style::Color::Reset,
            SeriColor::Red => crossterm::style::Color::Red,
            SeriColor::White => crossterm::style::Color::White,
            SeriColor::Yellow => crossterm::style::Color::Yellow,
        }
    }
}
