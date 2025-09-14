#![doc(html_root_url = "https://docs.rs/sericom-core/0.3.0")]
//! `sericom-core` is the underlying library for [`sericom`](https://crates.io/crates/sericom)
//!
//! As it sits right now, this library is largely meant to be solely used by `sericom`
//! directly. Therefore, it is not intended to be used within other projects/crates.
//!
//! If other projects develop a need to use this library within their projects, please
//! create an [issue](https://github.com/tkatter/sericom) so I can become aware and work
//! towards making `sericom-core` a generalized/compatible library that is better suited
//! for use among other crates.

pub mod cli;
pub mod configs;
pub mod debug;
pub mod screen_buffer;
pub mod serial_actor;

mod macros {
    //! This module holds generic macros that are used throughout sericom.

    /// Takes a [`&Path`][std::path::Path] and first checks whether it exists or if it is a
    /// directory.
    ///
    /// If it doesn't exist or is not a directory, it will create
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
}
