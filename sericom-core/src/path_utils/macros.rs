//! This module holds helper macros for dealing with paths

/// Takes a [`&Path`][std::path::Path] and first checks whether it exists or if it is a
/// directory. If it doesn't exist or is not a directory, it will create
/// the directory recursively; creating the necessary parent directories.
///
/// ## Example
/// ```
/// use sericom_core::create_recursive;
/// use std::path::PathBuf;
/// fn mkdir() {
///     let path = PathBuf::from("some/dir");
///     create_recursive!(&path);
///     assert!(path.is_dir() && path.exists());
/// }
/// ```
#[macro_export]
macro_rules! create_recursive {
    ($path:expr) => {
        let create_recursive_dir = |p: &std::path::Path| {
            if !p.exists() || !p.is_dir() {
                let mut builder = std::fs::DirBuilder::new();
                builder.recursive(true);
                builder.create(p).expect("Recursive mode won't panic");
            }
        };

        create_recursive_dir($path)
    };
}

/// Used to add a `.map_err()` to function calls that return a `Result<T, E>`
/// to provide better context for the error and print it nicely to stdout.
///
/// Takes 2 arguements and optionally a third and fourth:
/// - The first argument is the expression or function call that would return a `Result<T, E>`
/// - The second argument is context that better describes the returned error
/// - The optional third argument is the 'USAGE: sericom ...' that would typically be printed by `clap`
///   for the respective command
/// - The optional fourth argument is an additional "help:" message
///
/// ## Example
/// ```
/// use serial2_tokio::SerialPort;
/// use crossterm::style::Stylize;
/// use sericom_core::map_miette;
/// fn returns_err() -> miette::Result<()> {
///     let baud: u32 = 9600;
///     let port = "/dev/fakeport";
///     let x = map_miette!(
///         SerialPort::open(port, baud),
///         format!("Failed to open port '{}'", port),
///         format!("{} {} [OPTIONS] [PORT] [COMMAND]",
///             "USAGE:".bold().underlined(),
///             "sericom".bold()
///         ),
///         help = format!(
///             "To see available ports, try `{}`.",
///             "sericom list-ports".bold().cyan()
///         )
///     )?;
///     Ok(())
/// }
/// let fn_err = returns_err();
/// assert!(fn_err.is_err());
/// ```
#[macro_export]
macro_rules! map_miette {
    // Clap-style USAGE: && additional "help" message
    ($expr:expr, $wrap_msg:expr, $usage:expr, help = $add_help:expr) => {
        $expr.map_err(|e| {
            use crossterm::style::Stylize;
            miette::miette!(
                help = format!("{}\nFor more information, try `sericom --help`.", $add_help),
                "{e}"
            )
            .wrap_err(format!("{}\n\n{}\n", $wrap_msg, $usage).red())
        })
    };

    // Clap-style USAGE: && default "help" message
    ($expr:expr, $wrap_msg:expr, $usage:expr) => {
        $expr.map_err(|e| {
            use crossterm::style::Stylize;
            miette::miette!(help = "For more information, try `sericom --help`.", "{e}")
                .wrap_err(format!("{}\n\n{}\n", $wrap_msg, $usage).red())
        })
    };

    // Additional "help" message
    ($expr:expr, $wrap_msg:expr, help = $add_help:expr) => {
        $expr.map_err(|e| {
            use crossterm::style::Stylize;
            miette::miette!(
                help = format!("{}\nFor more information, try `sericom --help`.", $add_help),
                "{e}"
            )
            .wrap_err(format!("{}", $wrap_msg).red())
        })
    };

    // Default "help" message
    ($expr:expr, $wrap_msg:expr) => {
        $expr.map_err(|e| {
            use crossterm::style::Stylize;
            miette::miette!(help = "For more information, try `sericom --help`.", "{e}")
                .wrap_err(format!("{}", $wrap_msg).red())
        })
    };
}

/// Creates the default filename if none is specified from the `port` name
/// and a timestamp.
///
/// Can also add a prefix the filename or the target out-dir.
/// Path will end up looking like this:
///  - Windows: `com4-09251554.txt`
///  - Unix: `ttyUSB0-09251554.txt`
///  - Prefixed: `trace-ttyUSB0-09251554.txt`
///
/// # Errors
/// Errors on Unix systems if [`file_name`] returns `None`
///
/// # Examples
/// ```
/// use sericom_core::compat_port_path;
/// use std::path::PathBuf;
/// use chrono::Utc;
///
/// fn get_default_fname() -> miette::Result<()> {
///     let out_dir = PathBuf::from("/home/dev/test");
///     let port_name = PathBuf::from("/dev/ttyUSB0");
///
///     let fname = compat_port_path!(port_name.clone(), prefix = "test");
///     assert_eq!(fname, PathBuf::from(format!(
///         "testing-ttyUSB0-{}.txt",
///         Utc::now().format("%m%d%H%M")
///     )));
///
///     let fname = compat_port_path!(out_dir, port_name.clone(), prefix = "test");
///     assert_eq!(fname, PathBuf::from(format!(
///         "/home/dev/test/testing-ttyUSB0-{}.txt",
///         Utc::now().format("%m%d%H%M")
///     )));
///
///     let fname = compat_port_path!(out_dir, port_name.clone());
///     assert_eq!(fname, PathBuf::from(format!(
///         "/home/dev/test/ttyUSB0-{}.txt",
///         Utc::now().format("%m%d%H%M")
///     )));
///
///     let fname = compat_port_path!(port_name);
///     assert_eq!(fname, PathBuf::from(format!(
///         "ttyUSB0-{}.txt",
///         Utc::now().format("%m%d%H%M")
///     )));
///     Ok(())
/// }
///
///
/// ```
///
/// [`file_name`]: std::path::Path::file_name()
#[macro_export]
macro_rules! compat_port_path {
    ($port:expr, prefix = $prefix:literal) => {{
        use chrono;
        use std::path::PathBuf;

        let path_port = $crate::path_utils::get_compat_port_path($port)?;
        PathBuf::from(format!(
            "./{}-{}-{}.txt",
            $prefix,
            path_port.display(),
            chrono::Utc::now().format("%m%d%H%M"),
        ))
    }};

    ($out_dir:expr, $port:expr, prefix = $prefix:expr) => {{
        use chrono;

        let path_port = $crate::path_utils::get_compat_port_path($port)?;
        $out_dir.join(format!(
            "./{}-{}-{}.txt",
            $prefix,
            path_port.display(),
            chrono::Utc::now().format("%m%d%H%M"),
        ))
    }};

    ($out_dir:expr, $port:expr) => {{
        use chrono;

        let path_port = $crate::path_utils::get_compat_port_path($port)?;
        $out_dir.join(format!(
            "./{}-{}.txt",
            path_port.display(),
            chrono::Utc::now().format("%m%d%H%M"),
        ))
    }};

    ($port:expr) => {{
        use chrono;

        let path_port = $crate::path_utils::get_compat_port_path($port)?;
        PathBuf::from(format!(
            "./{}-{}.txt",
            path_port.display(),
            chrono::Utc::now().format("%m%d%H%M"),
        ))
    }};
}

#[doc(hidden)]
pub fn get_compat_port_path<S>(port: S) -> miette::Result<std::path::PathBuf>
where
    S: Into<std::path::PathBuf>,
{
    use miette::{self, WrapErr};
    use std::path::PathBuf;

    #[cfg(windows)]
    {
        Ok(port.into())
    }
    #[cfg(unix)]
    {
        let p: PathBuf = port.into();
        Ok(PathBuf::from(p.file_name().ok_or(std::io::ErrorKind::InvalidFilename)
            .map_err(|e| miette::miette!(
                help = format!("The name of the tracing file is tied to the port being opened, make sure you are using a valid port."),
                "{e}: '{}'\n",
                p.display()
            )).wrap_err_with(|| format!("Could not create file: '{}' for tracing output.\n", p.display()))?))
    }
}

/// Macro to join a path to the user's home directory and
/// check whether it exists.
///
/// Used for building the $XDG base directories.
///
/// Returns `None` if the joined path doesn't exist.
macro_rules! push_n_check {
    ($home:expr, $push:literal) => {
        $home.push($push);
        if !$home.exists() {
            return None;
        }
    };
}

/// Macro to expand a path like shell expansion.
///
/// - On unix, this handles the $XDG base directories and `~`
/// - On Windows, this handles %USERPROFILE%, %APPDATA%, etc.
///
/// Returns `None` if unable to retrieve the user's [`home_dir`]
///
/// [`home_dir`]: std::env::home_dir()
macro_rules! expand_path {
    ($self:ident, $expand:literal, to = $expand_to:literal) => {{
        use std::{env, path::PathBuf};

        if $self.starts_with($expand) {
            let mut home = env::home_dir()?;
            let expanded: PathBuf = $self.components().skip(1).collect();
            push_n_check!(home, $expand_to);
            $self = home.join(expanded);
        }
    }};

    ($self:ident, $expand:literal) => {{
        use std::{env, path::PathBuf};

        if $self.starts_with($expand) {
            let home = env::home_dir()?;
            let expanded: PathBuf = $self.components().skip(1).collect();
            $self = home.join(expanded);
        }
    }};
}

/// For use with Windows Env variables for path expansions
#[cfg(windows)]
macro_rules! expand_env_path {
    ($self:ident, $expand:literal, env_var = $env_var:literal) => {{
        use std::{env, path::PathBuf};

        if $self.starts_with($expand) {
            if let Ok(base_path) = env::var($env_var) {
                let expanded: PathBuf = $self.components().skip(1).collect();
                $self = PathBuf::from(base_path).join(expanded)
            }
        }
    }};
}

/// Trait for expanding paths in a shell-like way.
pub trait ExpandPaths {
    /// Expands path in a shell-like way
    ///
    /// Takes ownership of the implementor and returns `Some(PathBuf)` if
    /// self can successfully expand the path; otherwise, returns `None`.
    fn get_expanded_path(self) -> Option<std::path::PathBuf>;
}

impl ExpandPaths for std::path::PathBuf {
    /// Expands path in a shell-like way
    ///
    /// Returns `None` if fails to retrieve the user's home dir.
    #[cfg(unix)]
    fn get_expanded_path(mut self) -> Option<Self> {
        expand_path!(self, "~");
        expand_path!(self, "$HOME");
        expand_path!(self, "$XDG_CACHE_HOME", to = ".cache");
        expand_path!(self, "$XDG_CONFIG_HOME", to = ".config");
        expand_path!(self, "$XDG_DATA_HOME", to = ".local/share");
        expand_path!(self, "$XDG_DESKTOP_DIR", to = "Desktop");
        expand_path!(self, "$XDG_DOCUMENTS_DIR", to = "Documents");
        expand_path!(self, "$XDG_DOWNLOAD_DIR", to = "Downloads");
        expand_path!(self, "$XDG_MUSIC_DIR", to = "Music");
        expand_path!(self, "$XDG_PICTURES_DIR", to = "Pictures");
        expand_path!(self, "$XDG_PUBLICSHARE_DIR", to = "Public");
        expand_path!(self, "$XDG_STATE_HOME", to = ".local/state");
        expand_path!(self, "$XDG_TEMPLATES_DIR", to = "Templates");
        Some(self)
    }
    #[cfg(windows)]
    fn get_expanded_path(mut self) -> Option<Self> {
        // Basic home directory expansion
        expand_path!(self, "~");
        expand_env_path!(self, "%USERPROFILE%", env_var = "USERPROFILE");
        expand_env_path!(self, "%APPDATA%", env_var = "APPDATA");
        expand_env_path!(self, "%LOCALAPPDATA%", env_var = "LOCALAPPDATA");
        expand_env_path!(self, "%TEMP%", env_var = "TEMP");
        expand_env_path!(self, "%TMP%", env_var = "TMP");
        expand_path!(self, "%DESKTOP%", to = "Desktop");
        expand_path!(self, "%DOCUMENTS%", to = "Documents");
        expand_path!(self, "%DOWNLOADS%", to = "Downloads");
        expand_path!(self, "%MUSIC%", to = "Music");
        expand_path!(self, "%PICTURES%", to = "Pictures");
        expand_path!(self, "%VIDEOS%", to = "Videos");
        expand_path!(self, "%PUBLIC%", to = "Public");
        Some(self)
    }
}
