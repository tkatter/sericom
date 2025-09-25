//! This module handles the structuring, valid options, and parsing of user config
//! files. User config files must be `config.toml` and are parsed with [`serde`] and
//! respectively serde's [`toml`] crate.

mod appearance;
mod defaults;
pub mod errors;
pub use appearance::*;
pub use defaults::*;

use crate::{
    configs::errors::{ConfigError, TomlError},
    create_recursive,
};
use serde::Deserialize;
use std::{io::Read, ops::Range, path::PathBuf, sync::OnceLock};

/// Global value of the user's config.
///
/// Currently it is immutable after initialized, therefore any changes to the
/// underlying [`Config`] must be made before calling [`initialize_config()`].
///
/// To get a reference to the global config during runtime, call [`get_config()`].
pub static CONFIG: OnceLock<Config> = OnceLock::new();

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
        if let Some(script) = overrides.exit_script {
            self.defaults.exit_script = Some(script);
        }
    }
}

/// This function constructs a global `static CONFIG` for the rest of the program's
/// duration to provide a reference to the config for the remainder of the program.
///
/// It checks for the user's config file and if it doesn't exist, it will use
/// [`Config::default()`]. If the user's config does exist but does not set values
/// for every field, the global `static CONFIG` will be initialized with the user's
/// values and fill in the unspecified fields with their default values.
///
/// Takes [`ConfigOverride`] to set any overriding values before initialization.
///
/// Returns a [`ConfigError::AlreadyInitialized`] error if called after it has
/// already been called ([`CONFIG`] has already been set).
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

/// When called, [`get_config()`] returns a reference to the global [`CONFIG`]
/// that was initialized at the start of the program.
///
/// See [`Config`].
///
/// ## Panics
/// Will panic if [`CONFIG`] as not been initialized before calling with [`initialize_config()`].
pub fn get_config() -> &'static Config {
    CONFIG.get().expect("Config not initialized")
}

#[derive(Debug)]
/// Available configuration options that can be overridden
pub struct ConfigOverride {
    /// Overrides [`Appearance::fg`]
    pub color: Option<SeriColor>,
    /// Overrides [`Defaults::out_dir`]
    pub out_dir: Option<PathBuf>,
    /// Overrides [`Defaults::exit_script`]
    pub exit_script: Option<PathBuf>,
}

fn get_conf_dir() -> std::path::PathBuf {
    let mut user_home_dir = std::env::home_dir().expect("Failed to get home directory");

    if cfg!(windows) {
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
fn parse_test_config() -> miette::Result<()> {
    use miette::IntoDiagnostic;
    let file: Config = toml::from_str(
        r#"
            [appearance]
            fg = "dark-grey"
            bg = "red"

            [defaults]
            out-dir = "$HOME/.config"
            exit-script = "~/.local/bin/format-cisco"
            "#,
    )
    .into_diagnostic()?;

    let parsed_conf = Config {
        appearance: Appearance {
            fg: SeriColor::DarkGrey,
            bg: SeriColor::Red,
        },
        defaults: Defaults {
            out_dir: PathBuf::from("/home/thomas/.config"),
            exit_script: Some(PathBuf::from("/home/thomas/.local/bin/format-cisco")),
            debug_dir: PathBuf::from("/home/thomas/Code/Work/sericom/sericom-core"),
            // file_exit_script: None,
        },
    };

    assert_eq!(file, parsed_conf);
    Ok(())
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
fn get_expanded_path() {
    use crate::path_utils::ExpandPaths;

    let p = PathBuf::from("$HOME/.config/sericom/config.toml")
        .get_expanded_path()
        .unwrap();

    let p2 = PathBuf::from("~/.config/sericom/config.toml")
        .get_expanded_path()
        .unwrap();

    let p3 = PathBuf::from("$XDG_CONFIG_HOME/some/path")
        .get_expanded_path()
        .unwrap();

    assert_eq!(p, PathBuf::from("/home/thomas/.config/sericom/config.toml"));
    assert_eq!(
        p2,
        PathBuf::from("/home/thomas/.config/sericom/config.toml")
    );
    assert_eq!(p3, PathBuf::from("/home/thomas/.config/some/path"))
}

// #[test]
// fn initialize_conf() -> miette::Result<()> {
//     initialize_config(ConfigOverride { color: None, out_dir: None, file_exit_script: None, })?;
//     // assert_eq!(config, &Config::default())
//     Ok(())
// }
