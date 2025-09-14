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
/// ```
#[derive(Debug, Deserialize, PartialEq, Eq)]
pub struct Appearance {
    #[serde(default = "default_fg")]
    pub fg: SeriColor,
    #[serde(default = "default_bg")]
    pub bg: SeriColor,
}

const fn default_fg() -> SeriColor {
    SeriColor::Green
}
const fn default_bg() -> SeriColor {
    SeriColor::None
}

impl Default for Appearance {
    fn default() -> Self {
        Self {
            fg: SeriColor::Green,
            bg: SeriColor::None,
        }
    }
}

/// Uses [`Cow`] to normalize strings passed to it.
///
/// If the input is already normalized, it simply returns a [`Cow::Borrowed`],
/// else will remove '-', '_', and whitespace and transform to lowercase.
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

/// A list of the valid options for [`SeriColor`].
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
    /// Takes a `normalizer` to be used for normalizing `input`. Pairs nicely
    /// with [`NORMALIZER`] for the standard situations in which [`SeriColor`]
    /// would be parsed from (config.toml and as a cli argument). However, you
    /// may rather want to use a custom `normalizer` instead.
    ///
    /// # Errors
    /// Returns an error of [`VALID_SERICOLORS`] if unable to parse into [`SeriColor`]
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
            "black" => Ok(Self::Black),
            "blue" => Ok(Self::Blue),
            "cyan" => Ok(Self::Cyan),
            "darkblue" => Ok(Self::DarkBlue),
            "darkcyan" => Ok(Self::DarkCyan),
            "darkgreen" => Ok(Self::DarkGreen),
            "darkgrey" | "darkgray" => Ok(Self::DarkGrey),
            "darkmagenta" => Ok(Self::DarkMagenta),
            "darkred" => Ok(Self::DarkRed),
            "darkyellow" => Ok(Self::DarkYellow),
            "default" => Ok(Self::None),
            "green" => Ok(Self::Green),
            "grey" | "gray" => Ok(Self::Grey),
            "magenta" => Ok(Self::Magenta),
            "red" => Ok(Self::Red),
            "white" => Ok(Self::White),
            "yellow" => Ok(Self::Yellow),
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
        match Self::parse_from_str(&s, NORMALIZER) {
            Ok(s) => Ok(s),
            Err(valid_colors) => Err(serde::de::Error::unknown_variant(&s, valid_colors)),
        }
    }
}

impl From<&SeriColor> for crossterm::style::Color {
    fn from(value: &SeriColor) -> Self {
        match value {
            SeriColor::Black => Self::Black,
            SeriColor::Blue => Self::Blue,
            SeriColor::Cyan => Self::Cyan,
            SeriColor::DarkBlue => Self::DarkBlue,
            SeriColor::DarkCyan => Self::DarkCyan,
            SeriColor::DarkGreen => Self::DarkGreen,
            SeriColor::DarkGrey => Self::DarkGrey,
            SeriColor::DarkMagenta => Self::DarkMagenta,
            SeriColor::DarkRed => Self::DarkRed,
            SeriColor::DarkYellow => Self::DarkYellow,
            SeriColor::Green => Self::Green,
            SeriColor::Grey => Self::Grey,
            SeriColor::Magenta => Self::Magenta,
            SeriColor::None => Self::Reset,
            SeriColor::Red => Self::Red,
            SeriColor::White => Self::White,
            SeriColor::Yellow => Self::Yellow,
        }
    }
}
