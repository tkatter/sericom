pub mod cli;
pub mod configs;
pub mod debug;
pub mod screen_buffer;
pub mod serial_actor;

mod macros {
    //! This module holds generic macros that are used throughout sericom.

    /// Takes a [`&Path`][std::path::Path] and first checks whether it exists or if it is a
    /// directory. If it doesn't exist or is not a directory, it will create
    /// the directory recursively; creating the necessary parent directories.
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
    /// Takes 3 arguements and optionally a fourth:
    /// - The first argument is the expression or function call that would return a `Result<T, E>`
    /// - The second argument is context that better describes the returned error
    /// - The third argument is the 'USAGE: sericom <ARG>' that would typically be printed by `clap`
    ///   for the respective command
    /// - The optional fourth argument is an additional "help:" message
    ///
    /// ## Example
    /// ```
    /// use serial2_tokio::SerialPort;
    /// use crossterm::style::Stylize;
    /// use sericom::map_miette;
    /// fn returns_err() -> miette::Result<()> {
    ///     let baud: u32 = 9600;
    ///     let port = "/dev/fakeport";
    ///     let x = map_miette!(
    ///         SerialPort::open(port, baud),
    ///         format!("Failed to open port '{}'", port),
    ///         "[OPTIONS] [PORT] [COMMAND]",
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
        // Additional "help" message
        ($expr:expr, $wrap_msg:expr, $usage:expr, help = $add_help:expr) => {
            $expr.map_err(|e| {
                use crossterm::style::Stylize;
                let usage = format!(
                    "{} {} {}",
                    "USAGE:".bold().underlined(),
                    "sericom".bold(),
                    $usage
                );
                miette::miette!(
                    help = format!("{}\nFor more information, try `sericom --help`.", $add_help),
                    "{e}"
                )
                .wrap_err(format!("{}\n\n{}\n", $wrap_msg, usage).red())
            })
        };

        // Default "help" message
        ($expr:expr, $wrap_msg:expr, $usage:expr) => {
            $expr.map_err(|e| {
                use crossterm::style::Stylize;
                let usage = format!(
                    "{} {} {}",
                    "USAGE:".bold().underlined(),
                    "sericom".bold(),
                    $usage
                );
                miette::miette!(help = "For more information, try `sericom --help`.", "{e}")
                    .wrap_err(format!("{}\n\n{}\n", $wrap_msg, usage).red())
            })
        };
    }
}
