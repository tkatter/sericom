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

#[macro_export]
macro_rules! compat_port_path {
    ($port:expr, prefix = $prefix:literal) => {{
        use chrono;

        let path_port = $crate::path_utils::get_compat_port_path($port)?;
        PathBuf::from(format!(
            "./{}-{}-{}.txt",
            $prefix,
            path_port.display(),
            chrono::Utc::now().format("%m%d%H%M"),
        ))
    }};

    ($out_dir:expr, $port:expr, prefix = $prefix:expr) => {{
        use super::get_compat_port_path;
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

    let path_port: PathBuf = if cfg!(windows) {
        port.into()
    } else {
        let p: PathBuf = port.into();
        PathBuf::from(p.file_name().ok_or(std::io::ErrorKind::InvalidFilename)
            .map_err(|e| miette::miette!(
                help = format!("The name of the tracing file is tied to the port being opened, make sure you are using a valid port."),
                "{e}: '{}'\n",
                p.display()
            )).wrap_err_with(|| format!("Could not create file: '{}' for tracing output.\n", p.display()))?)
    };

    Ok(path_port)
}

// TODO: DOCS
macro_rules! push_n_check {
    ($home:expr, $push:literal) => {
        $home.push($push);
        if !$home.exists() {
            return Err(());
        }
    };
}

// TODO: DOCS
macro_rules! expand_path {
    ($self:ident, $expand:literal, to = $expand_to:literal) => {{
        use std::{env, ffi::OsStr, os::unix::ffi::OsStrExt, path::PathBuf};

        if $self.starts_with($expand) {
            let Some(mut home) = env::home_dir() else {
                return Err(());
            };
            let expanded: PathBuf = $self
                .components()
                .filter(|c| c.as_os_str() != OsStr::from_bytes($expand.as_bytes()))
                .collect();
            push_n_check!(home, $expand_to);
            $self = home.join(expanded);
        }
    }};
    ($self:ident, $expand:literal) => {{
        use std::{env, ffi::OsStr, os::unix::ffi::OsStrExt, path::PathBuf};

        if $self.starts_with($expand) {
            let Some(home) = env::home_dir() else {
                return Err(());
            };
            let expanded: PathBuf = $self
                .components()
                .filter(|c| c.as_os_str() != OsStr::from_bytes($expand.as_bytes()))
                .collect();
            $self = home.join(expanded);
        }
    }};
}

pub(crate) trait ExpandUnixPaths {
    fn get_expanded_path(self) -> Result<std::path::PathBuf, ()>;
}

impl ExpandUnixPaths for std::path::PathBuf {
    fn get_expanded_path(mut self) -> Result<Self, ()> {
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
        Ok(self)
    }
}
