use serde::Deserialize;
use std::{io::Read, sync::OnceLock};

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

pub fn initialize_config() -> Result<(), Box<dyn std::error::Error>> {
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
        .map_err(|_| "Config already initialized")?;
    Ok(())
}

pub fn get_config() -> &'static Config {
    CONFIG.get().expect("Config not initialized")
}

fn get_conf_dir() -> std::io::Result<std::path::PathBuf> {
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
fn parse_file() {
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
    let check = get_conf_dir();
    assert!(check.is_ok())
}

#[test]
fn check_conf_dir_is_dir() {
    let dir = get_conf_dir().unwrap();

    assert!(std::fs::metadata(dir).unwrap().is_dir())
}

#[test]
fn verify_conf_dir() {
    let dir = get_conf_dir().unwrap();
    assert_eq!(
        dir.as_path(),
        std::path::Path::new("/home/thomas/.config/sericom")
    )
}

#[test]
fn initialize_conf() {
    initialize_config().unwrap();
    let config = get_config();

    assert_eq!(config, &Config::default())
}
