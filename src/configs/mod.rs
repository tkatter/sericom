//! This module handles the structuring, valid options, and parsing of user config
//! files. User config files must be `config.toml` and are parsed with [`serde`] and
//! respectively serde's [`toml`] crate.

use serde::Deserialize;
use std::{io::Read, ops::Range, sync::OnceLock};

pub mod errors;

use crate::{
    configs::errors::{ConfigError, TomlError},
    create_recursive,
};

/// A wrapper around [`crossterm::style::Color`] to allow for implementing serde's
/// [`Deserialize`] beyond the default implementation from `#[derive(Deserialize)]`
#[derive(Debug, PartialEq)]
pub enum SeriColor {
    DarkGrey,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
    Black,
    DarkRed,
    DarkGreen,
    DarkYellow,
    DarkBlue,
    DarkMagenta,
    DarkCyan,
    Grey,
    None,
}

impl<'de> Deserialize<'de> for SeriColor {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let normalizer = |s: &str| -> String { s.to_lowercase().replace(['-', '_', ' '], "") };
        let s = String::deserialize(deserializer)?;
        let normalized = normalizer(&s);
        match normalized.as_str() {
            "darkgrey" | "darkgray" => Ok(SeriColor::DarkGrey),
            "red" => Ok(SeriColor::Red),
            "green" => Ok(SeriColor::Green),
            "yellow" => Ok(SeriColor::Yellow),
            "blue" => Ok(SeriColor::Blue),
            "magenta" => Ok(SeriColor::Magenta),
            "cyan" => Ok(SeriColor::Cyan),
            "white" => Ok(SeriColor::White),
            "black" => Ok(SeriColor::Black),
            "darkred" => Ok(SeriColor::DarkRed),
            "darkgreen" => Ok(SeriColor::DarkGreen),
            "darkyellow" => Ok(SeriColor::DarkYellow),
            "darkblue" => Ok(SeriColor::DarkBlue),
            "darkmagenta" => Ok(SeriColor::DarkMagenta),
            "darkcyan" => Ok(SeriColor::DarkCyan),
            "grey" | "gray" => Ok(SeriColor::Grey),
            "default" => Ok(SeriColor::None),
            _ => Err(serde::de::Error::unknown_variant(
                &s,
                &[
                    "grey",
                    "dark-cyan",
                    "dark-magenta",
                    "dark-blue",
                    "dark-yellow",
                    "dark-green",
                    "dark-red",
                    "black",
                    "white",
                    "cyan",
                    "magenta",
                    "blue",
                    "yellow",
                    "green",
                    "red",
                    "dark-grey",
                    "default",
                ],
            )),
        }
    }
}

impl From<&SeriColor> for crossterm::style::Color {
    fn from(value: &SeriColor) -> crossterm::style::Color {
        match value {
            SeriColor::DarkGrey => crossterm::style::Color::DarkGrey,
            SeriColor::Red => crossterm::style::Color::Red,
            SeriColor::Green => crossterm::style::Color::Green,
            SeriColor::Yellow => crossterm::style::Color::Yellow,
            SeriColor::Blue => crossterm::style::Color::Blue,
            SeriColor::Magenta => crossterm::style::Color::Magenta,
            SeriColor::Cyan => crossterm::style::Color::Cyan,
            SeriColor::White => crossterm::style::Color::White,
            SeriColor::Black => crossterm::style::Color::Black,
            SeriColor::DarkRed => crossterm::style::Color::DarkRed,
            SeriColor::DarkGreen => crossterm::style::Color::DarkGreen,
            SeriColor::DarkYellow => crossterm::style::Color::DarkYellow,
            SeriColor::DarkBlue => crossterm::style::Color::DarkBlue,
            SeriColor::DarkMagenta => crossterm::style::Color::DarkMagenta,
            SeriColor::DarkCyan => crossterm::style::Color::DarkCyan,
            SeriColor::Grey => crossterm::style::Color::Grey,
            SeriColor::None => crossterm::style::Color::Reset,
        }
    }
}

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

/// Represents the `[defaults]` table of the `config.toml` file.
///
/// The `[defaults]` table holds configuration values for how sericom
/// should behave. Currently the user may only specify a default `out_dir`,
/// where files will be created when running `sericom -f path/to/file [PORT]`.
///
/// The default values (if no config exists) is the current directory:
/// ```toml
/// [defaults]
/// out_dir = "./"
/// ```
#[derive(Debug, Deserialize, PartialEq)]
pub struct Defaults {
    #[serde(default = "default_out_dir")]
    pub out_dir: String,
}

impl Default for Defaults {
    fn default() -> Self {
        Self {
            out_dir: "./".to_string(),
        }
    }
}

fn default_out_dir() -> String {
    "./".to_string()
}

/// Represents the entire `config.toml` configuration file.
///
/// See [`Appearance`] and [`Defaults`]
#[derive(Default, Debug, Deserialize, PartialEq)]
pub struct Config {
    #[serde(default)]
    pub appearance: Appearance,
    #[serde(default)]
    pub defaults: Defaults,
}

static CONFIG: OnceLock<Config> = OnceLock::new();

/// This function constructs a `static CONFIG` for the rest of sericom to get a
/// reference to throughout the remainder of the program.
///
/// It checks for the user's config file and if it doesn't exist, it will use
/// [`Config::default()`]. If the user's config does exist but does not set values
/// for every field, the global `static CONFIG` will be initialized with the user's
/// values and fill in the unspecified fields with their default values.
pub fn initialize_config() -> miette::Result<(), ConfigError> {
    let config: Config = if let Ok(config_file) = get_config_file() {
        let mut file = std::fs::File::open(config_file).expect("File should exist");
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        toml::from_str(&contents).map_err(|e| {
            TomlError::new(
                e.span().unwrap_or(Range { start: 0, end: 0 }),
                contents,
                e.message().to_string(),
            )
        })?
    } else {
        Config::default()
    };

    CONFIG
        .set(config)
        .map_err(|_| ConfigError::AlreadyInitialized)?;
    Ok(())
}

/// When called, `get_config()` returns a reference to the global `static CONFIG`
/// that was initialized at the start of the program.
///
/// See [`Config`].
pub fn get_config() -> &'static Config {
    CONFIG.get().expect("Config not initialized")
}

fn get_conf_dir() -> std::path::PathBuf {
    let mut user_home_dir = std::env::home_dir().expect("Failed to get home directory");

    if cfg!(target_os = "windows") {
        user_home_dir.push(".config\\sericom");
    } else {
        user_home_dir.push(".config/sericom");
    }

    let user_conf_dir = user_home_dir;
    create_recursive!(user_conf_dir.as_path());

    user_conf_dir
}

fn get_config_file() -> miette::Result<std::path::PathBuf, ConfigError> {
    let mut conf_dir = get_conf_dir();
    conf_dir.push("config.toml");
    let conf_file = conf_dir;

    if conf_file.exists() && conf_file.is_file() {
        Ok(conf_file)
    } else {
        Err(std::io::Error::new(std::io::ErrorKind::NotFound, "Could not find config file.").into())
    }
}

#[test]
fn parse_test_config() {
    let file: Config = toml::from_str(
        r#"
            [appearance]
            fg = "dark-grey"
            bg = "red"
            hl_fg = "white"
            hl_bg = "blue"

            [defaults]
            out_dir = "$HOME/.configs"
            "#,
    )
    .unwrap();

    let parsed_conf = Config {
        appearance: Appearance {
            fg: SeriColor::DarkGrey,
            bg: SeriColor::Red,
            hl_fg: SeriColor::White,
            hl_bg: SeriColor::Blue,
        },
        defaults: Defaults {
            out_dir: "$HOME/.configs".to_string(),
        },
    };

    assert_eq!(file, parsed_conf)
}

#[test]
fn check_conf_dir_is_dir() {
    let dir = get_conf_dir();
    assert!(std::fs::metadata(dir).unwrap().is_dir())
}

#[test]
fn valid_conf_dir() {
    let dir = get_conf_dir();
    if cfg!(target_family = "windows") {
        assert_eq!(dir.to_str().unwrap(), "C:\\Users\\Thomas\\.config\\sericom")
    } else {
        assert_eq!(dir.to_str().unwrap(), "/home/thomas/.config/sericom")
    }
}

#[test]
fn initialize_conf() {
    initialize_config().unwrap();
    let config = get_config();
    assert_eq!(config, &Config::default())
}
