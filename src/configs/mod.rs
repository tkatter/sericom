use serde::Deserialize;
use std::{io::Read, sync::OnceLock};

use crate::create_recursive;

#[derive(Debug)]
pub enum ConfigError {
    IoError(std::io::Error),
    TomlError(toml::de::Error),
    AlreadyInitialized,
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AlreadyInitialized => write!(f, "Config is already initialized"),
            Self::IoError(e) => write!(f, "{e}"),
            Self::TomlError(e) => write!(f, "{e}"),
        }
    }
}

impl From<toml::de::Error> for ConfigError {
    fn from(value: toml::de::Error) -> Self {
        Self::TomlError(value)
    }
}

impl From<std::io::Error> for ConfigError {
    fn from(value: std::io::Error) -> Self {
        Self::IoError(value)
    }
}

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

#[derive(Default, Debug, Deserialize, PartialEq)]
pub struct Config {
    #[serde(default)]
    pub appearance: Appearance,
    #[serde(default)]
    pub defaults: Defaults,
}

static CONFIG: OnceLock<Config> = OnceLock::new();

pub fn initialize_config() -> Result<(), ConfigError> {
    let config: Config = if let Ok(config_file) = get_config_file() {
        let mut file = std::fs::File::open(config_file).expect("File should exist");
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        toml::from_str(&contents)?
    } else {
        Config::default()
    };

    CONFIG
        .set(config)
        .map_err(|_| ConfigError::AlreadyInitialized)?;
    Ok(())
}

pub fn get_config() -> &'static Config {
    CONFIG.get().expect("Config not initialized")
}

fn get_conf_dir() -> std::io::Result<std::path::PathBuf> {
    let mut user_home_dir = std::env::home_dir().expect("Failed to get home directory");

    if cfg!(target_os = "windows") {
        user_home_dir.push(".config\\sericom");
    } else {
        user_home_dir.push(".config/sericom");
    }

    let user_conf_dir = user_home_dir;
    create_recursive!(user_conf_dir.as_path());

    Ok(user_conf_dir)
}

fn get_config_file() -> std::io::Result<std::path::PathBuf> {
    let mut conf_dir = get_conf_dir()?;
    conf_dir.push("config.toml");
    let conf_file = conf_dir;

    if conf_file.exists() && conf_file.is_file() {
        Ok(conf_file)
    } else {
        Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Could not find config file.",
        ))
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
fn check_conf_dir_is_ok() {
    let check = get_conf_dir();
    assert!(check.is_ok())
}

#[test]
fn check_conf_dir_is_dir() {
    let dir = get_conf_dir().unwrap();
    assert!(std::fs::metadata(dir).unwrap().is_dir())
}

#[test]
fn valid_conf_dir() {
    let dir = get_conf_dir().unwrap();
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
