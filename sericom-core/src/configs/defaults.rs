use crate::path_utils::{ExpandPaths, is_executable};
use serde::{Deserialize, Deserializer};
use std::path::PathBuf;

/// Represents the `[defaults]` table of the `config.toml` file.
///
/// The `[defaults]` table holds configuration values for how sericom
/// should behave. Currently the user may only specify a default `out_dir`,
/// where files will be created when running `sericom -f path/to/file [PORT]`.
///
/// The default values (if no config exists) is the current directory:
/// ```toml
/// [defaults]
/// out-dir = "./"
/// exit-script = "/path/to/script"
/// ```
#[derive(Debug, Deserialize, PartialEq)]
pub struct Defaults {
    #[serde(rename = "out-dir")]
    #[serde(default = "default_out_dir")]
    #[serde(deserialize_with = "validate_dir")]
    pub out_dir: PathBuf,
    #[serde(rename = "exit-script")]
    #[serde(default)]
    #[serde(deserialize_with = "is_script")]
    pub exit_script: Option<PathBuf>,
}

impl Default for Defaults {
    fn default() -> Self {
        Self {
            out_dir: PathBuf::from("./"),
            exit_script: None,
        }
    }
}

fn default_out_dir() -> PathBuf {
    PathBuf::from("./")
}

fn validate_dir<'de, D>(deserializer: D) -> Result<PathBuf, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::Error;

    let p = PathBuf::deserialize(deserializer)?
        .get_expanded_path()
        .ok_or(Error::custom("Error expanding path."))?;
    if !p.exists() || !p.is_dir() {
        return Err(serde::de::Error::custom(
            "Error setting out-dir, Either does not exist or is not a directory",
        ));
    }
    Ok(p)
}

fn is_script<'de, D>(deserializer: D) -> Result<Option<PathBuf>, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::Error;

    let p = PathBuf::deserialize(deserializer)?
        .get_expanded_path()
        .ok_or(Error::custom("Error expanding path."))?;
    if !p.exists() || !p.is_file() {
        return Err(serde::de::Error::custom(
            "Error retrieving file, Either does not exist or is not a file",
        ));
    }

    if !is_executable(&p) {
        return Err(serde::de::Error::custom(
            "Invalid file type, Make sure the file is executable",
        ));
    }

    Ok(Some(p))
}
