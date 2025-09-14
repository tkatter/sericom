use serde::Deserialize;

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
#[derive(Debug, Deserialize, PartialEq, Eq)]
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
