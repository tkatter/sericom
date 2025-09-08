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
use tracing::{event, span, Level};
use std::io::{self, Write};

#[derive(Parser)]
#[command(name = "sericom", version, about, long_about = None)]
#[command(next_line_help = true)]
#[command(propagate_version = true)]
struct Cli {
    /// The path to a serial port.
    ///
    /// For Linux/MacOS something like `/dev/tty1`, Windows `COM1`.
    /// To see available ports, use `sericom list-ports`.
    port: Option<String>,
    /// Baud rate for the serial connection.
    ///
    /// To see a list of valid baud rates, use `sericom list-bauds`.
    #[arg(short, long, value_parser = valid_baud_rate, default_value_t = 9600)]
    baud: u32,
    /// Path to a file for the output.
    #[arg(short, long)]
    file: Option<String>,
    /// Display debug output
    #[arg(short, long)]
    debug: bool,
    #[command(subcommand)]
    command: Option<Commands>,
}

#[allow(clippy::enum_variant_names)]
#[derive(Subcommand)]
enum Commands {
    /// Lists all valid baud rates
    ListBauds,
    /// Lists all available serial ports
    ListPorts,
    /// Gets the settings for a serial port
    ListSettings {
        #[arg(short, long, value_parser = valid_baud_rate, default_value_t = 9600)]
        baud: u32,
        /// Path to the port to open
        #[arg(short, long)]
        port: String,
    },
}

#[tokio::main]
async fn main() -> miette::Result<()> {
    let file = std::fs::File::options()
        .write(true)
        .create(true)
        .truncate(true)
        .open("./tracing.txt")
        .into_diagnostic()?;

    let (non_blocking, _guard) = tracing_appender::non_blocking(file);
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(Level::DEBUG)
        .with_writer(non_blocking)
        .finish();
    tracing::subscriber::set_global_default(subscriber).into_diagnostic().wrap_err("Failed to set subscriber")?;

    let cli = Cli::parse();

    let span = span!(Level::TRACE, "Main");
    let _enter = span.enter();

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

    if let Some(port) = cli.port {
        event!(Level::TRACE, "opening connection");
        let connection = open_connection(cli.baud, &port)?;
        initialize_config()?;
        interactive_session(connection, cli.file, cli.debug, &port).await?;
    } else if let Some(cmd) = cli.command {
        match cmd {
            Commands::ListBauds => {
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
            Commands::ListPorts => {
                event!(Level::INFO, "listing ports");
                list_serial_ports()?;
            }
            Commands::ListSettings { baud, port } => {
                get_settings(baud, &port)?;
            }
        }
    }
    Ok(())
}
