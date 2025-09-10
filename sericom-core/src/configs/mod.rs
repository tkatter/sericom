//! This module handles the structuring, valid options, and parsing of user config
//! files. User config files must be `config.toml` and are parsed with [`serde`] and
//! respectively serde's [`toml`] crate.

pub mod errors;
mod appearance;
mod defaults;
pub use defaults::*;
pub use appearance::*;

use crate::{
    configs::errors::{ConfigError, TomlError},
    create_recursive,
};
use serde::Deserialize;
use std::{io::Read, ops::Range, sync::OnceLock};

static CONFIG: OnceLock<Config> = OnceLock::new();

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

impl Config {
    fn apply_overrides(&mut self, overrides: ConfigOverride) {
        if let Some(color) = overrides.color {
            self.appearance.fg = color;
        }
        if let Some(dir) = overrides.out_dir {
            self.defaults.out_dir = dir;
        }
    }
}

/// This function constructs a `static CONFIG` for the rest of sericom to get a
/// reference to throughout the remainder of the program.
///
/// It checks for the user's config file and if it doesn't exist, it will use
/// [`Config::default()`]. If the user's config does exist but does not set values
/// for every field, the global `static CONFIG` will be initialized with the user's
/// values and fill in the unspecified fields with their default values.
pub fn initialize_config(overrides: ConfigOverride) -> miette::Result<(), ConfigError> {
    let mut config: Config = if let Ok(config_file) = get_config_file() {
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

    config.apply_overrides(overrides);

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

#[derive(Debug)]
pub struct ConfigOverride {
    pub color: Option<SeriColor>,
    pub out_dir: Option<String>,
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

// #[test]
// fn initialize_conf() {
//     initialize_config().unwrap();
//     let config = get_config();
//     assert_eq!(config, &Config::default())
// }
