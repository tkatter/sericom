//! Sericom is a CLI tool for communicating with devices over a serial connection.
//!
//! Currently, Sericom runs similarily to another CLI tool called 'screen'. In the future,
//! Sericom plans to allow for users to create config files for customizing appearances
//! and defaults. Sericom also plans to allow the writing of custom scripts (similar to
//! expect scripts) that can be parsed and executed by Sericom. The intention of these
//! scripts is to be able to automate tasks that take place over a serial connection i.e.
//! configuration, resetting, getting statistics, etc.

use clap::{CommandFactory, Parser, Subcommand};
use crossterm::style::Stylize;
use miette::{Context, IntoDiagnostic};
use sericom_core::{
    cli::{get_settings, interactive_session, list_serial_ports, open_connection, valid_baud_rate},
    configs::initialize_config,
};
use std::{
    io::{self, Write},
    path::PathBuf,
};

// #[command(group(
//     clap::ArgGroup::new("config_override")
//         .args(&["color"])
//         .requires_all(&["port", "baud"])
// ))]
#[derive(Parser)]
#[command(name = "sericom", version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    /// The path to a serial port.
    ///
    /// For Linux/MacOS something like `/dev/tty1`, Windows `COM1`.
    port: Option<String>,
    /// Baud rate for the serial connection.
    #[arg(short, long, value_parser = valid_baud_rate, default_value_t = 9600)]
    baud: u32,
    #[clap(flatten)]
    config_override: ConfigOverrides,
    /// Path to a file for the output.
    #[arg(short, long)]
    file: Option<PathBuf>,
    /// Display debug output
    #[arg(short, long)]
    debug: bool,
    #[command(subcommand)]
    command: Option<Commands>,
}

#[allow(clippy::enum_variant_names)]
#[derive(Subcommand)]
enum Commands {
    /// Lists valid baud rates
    Bauds,
    /// Lists all available serial ports
    Ports,
    /// Gets the settings for a serial port
    Settings {
        #[arg(short, long, value_parser = valid_baud_rate, default_value_t = 9600)]
        baud: u32,
        /// Path to the port to open
        #[arg(short, long)]
        port: String,
    },
}

#[derive(Parser, Debug)]
struct ConfigOverrides {
    /// Set the forground color for the text
    #[arg(short, long, requires_all = &["port"], value_parser = color_parser)]
    color: Option<sericom_core::configs::SeriColor>,
    /// Override the `out_dir` for the file
    ///
    /// Alternatively could simply use the absolute path
    #[arg(short, long, requires_all = &["port", "file"])]
    out_dir: Option<String>,
}

impl From<ConfigOverrides> for sericom_core::configs::ConfigOverride {
    fn from(overrides: ConfigOverrides) -> Self {
        sericom_core::configs::ConfigOverride {
            color: overrides.color,
            out_dir: overrides.out_dir,
        }
    }
}

#[tokio::main]
async fn main() -> miette::Result<()> {
    let cli = Cli::parse();

    let file = std::fs::File::options()
        .write(true)
        .create(true)
        .truncate(true)
        .open("./tracing.txt")
        .into_diagnostic()?;
    let (non_blocking, _guard) = tracing_appender::non_blocking(file);
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .with_writer(non_blocking)
        // .without_time()
        .with_line_number(false)
        .with_target(false)
        .finish();
    tracing::subscriber::set_global_default(subscriber)
        .into_diagnostic()
        .wrap_err("Failed to set subscriber")?;

    if cli.port.is_none() && cli.command.is_none() {
        let mut cmd = Cli::command();
        cmd.error(
            clap::error::ErrorKind::MissingRequiredArgument,
            "Missing either PORT or COMMAND.",
        )
        .exit();
    }

    if cli.port.is_some() && cli.command.is_some() {
        let mut cmd = Cli::command();
        cmd.error(
            clap::error::ErrorKind::ArgumentConflict,
            "Must specify either PORT or SUBCOMMAND, not both.",
        )
        .exit();
    }

    if let Some(ref port) = cli.port {
        let connection = open_connection(cli.baud, port)?;
        let overrides: sericom_core::configs::ConfigOverride = cli.config_override.into();

        if let Some(path) = &cli.file
            && path.is_dir()
        {
            return Err(miette::miette!(
                "Could not create file at: '{}' because it is a directory.",
                path.display()
            ));
        }

        initialize_config(overrides)?;
        interactive_session(connection, cli.file, cli.debug, port).await?;
    } else if let Some(cmd) = cli.command {
        match cmd {
            Commands::Bauds => {
                let mut stdout = io::stdout();
                write!(stdout, "Valid baud rates:\r\n")
                    .into_diagnostic()
                    .wrap_err("Failed to write to stdout.".red())?;
                for baud in serial2_tokio::COMMON_BAUD_RATES {
                    write!(stdout, "{baud}\r\n")
                        .into_diagnostic()
                        .wrap_err("Failed to write to stdout.".red())?;
                }
            }
            Commands::Ports => {
                list_serial_ports()?;
            }
            Commands::Settings { baud, port } => {
                get_settings(baud, &port)?;
            }
        }
    }
    Ok(())
}

fn color_parser(input: &str) -> Result<sericom_core::configs::SeriColor, String> {
    use sericom_core::configs::{NORMALIZER, SeriColor};
    match SeriColor::parse_from_str(input, NORMALIZER) {
        Ok(c) => Ok(c),
        Err(valid_colors) => Err(format!("\n\nExpected one of: {}", valid_colors.join(", "))),
    }
}
