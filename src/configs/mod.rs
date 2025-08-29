use serde::Deserialize;
use std::{io::Read, sync::OnceLock};

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

#[derive(Debug, Deserialize, PartialEq)]
pub struct Appearance {
    pub text_fg: Option<String>,
    pub text_bg: Option<String>,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct Defaults {
    pub out_dir: String,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct Config {
    pub appearance: Appearance,
    pub defaults: Defaults,
}

static CONFIG: OnceLock<Config> = OnceLock::new();

impl Default for Config {
    fn default() -> Self {
        Self {
            appearance: Appearance {
                text_fg: Some("green".to_string()),
                text_bg: None,
            },
            defaults: Defaults {
                out_dir: "./".to_string(),
            },
        }
    }
}

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

#[cfg(target_family = "unix")]
fn get_conf_dir_unix() -> std::io::Result<std::path::PathBuf> {
    let mut user_home_dir = std::env::home_dir().expect("Failed to get home directory");
    user_home_dir.push(".config/sericom");
    let user_conf_dir = user_home_dir;

    if !user_conf_dir.is_dir() {
        let mut builder = std::fs::DirBuilder::new();
        builder.recursive(true);
        builder.create(&user_conf_dir)?;
    }

    Ok(user_conf_dir)
}

#[cfg(target_family = "windows")]
fn get_conf_dir_win() -> std::io::Result<std::path::PathBuf> {
    let mut user_home_dir = std::env::home_dir().expect("Failed to get home directory");
    user_home_dir.push(".config\\sericom");
    let user_conf_dir = user_home_dir;

    if !user_conf_dir.is_dir() {
        let mut builder = std::fs::DirBuilder::new();
        builder.recursive(true);
        builder.create(&user_conf_dir)?;
    }

    Ok(user_conf_dir)
}

fn get_config_file() -> std::io::Result<std::path::PathBuf> {
    #[cfg(target_family = "unix")]
    let mut conf_dir = get_conf_dir_unix()?;
    #[cfg(target_family = "windows")]
    let mut conf_dir = get_conf_dir_win()?;

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
            text_fg = "green"

            [defaults]
            out_dir = "$HOME/.configs"
            "#,
    )
    .unwrap();

    let parsed_conf = Config {
        appearance: Appearance {
            text_fg: Some("green".to_string()),
            text_bg: None,
        },
        defaults: Defaults {
            out_dir: "$HOME/.configs".to_string(),
        },
    };

    assert_eq!(file, parsed_conf)
}

#[test]
fn check_conf_dir_is_ok() {
    #[cfg(target_family = "unix")]
    let check = get_conf_dir_unix();
    #[cfg(target_family = "windows")]
    let check = get_conf_dir_win();
    assert!(check.is_ok())
}

#[test]
fn check_conf_dir_is_dir() {
    #[cfg(target_family = "unix")]
    let dir = get_conf_dir_unix().unwrap();
    #[cfg(target_family = "windows")]
    let dir = get_conf_dir_win().unwrap();

    assert!(std::fs::metadata(dir).unwrap().is_dir())
}

#[test]
fn initialize_conf() {
    initialize_config().unwrap();
    let config = get_config();

    assert_eq!(config, &Config::default())
}
