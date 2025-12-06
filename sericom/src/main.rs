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
    cli::{
        color_parser, get_settings, interactive_session, list_serial_ports, open_connection,
        valid_baud_rate,
    },
    configs::{get_config, initialize_config},
    path_utils::{is_script, validate_dir},
};
use std::{
    fmt::Display,
    io::{self, Write},
    path::{Path, PathBuf},
};

const RED: &str = "\x1b\x5b31m";
const BOLD: &str = "\x1b\x5b1m";
const UNBOLD: &str = "\x1b\x5b22m";
const RESET: &str = "\x1b\x5b0m";

#[derive(Parser)]
#[command(name = "sericom", version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[allow(clippy::enum_variant_names)]
#[derive(Subcommand)]
enum Commands {
    /// Connect to a serial port
    #[command(alias = "c")]
    Connect {
        /// The path to a serial port.
        ///
        /// For Linux/MacOS something like `/dev/ttyUSB0`, Windows `COM1`.
        port: String,
        /// Baud rate for the serial connection.
        #[arg(short, long, value_parser = valid_baud_rate, default_value_t = 9600)]
        baud: u32,
        #[clap(flatten)]
        config_override: ConfigOverrides,
        /// Path to a file for the output.
        #[arg(short, long)]
        file: Option<Option<PathBuf>>,
        /// Display debug output
        #[arg(short, long)]
        debug: bool,
    },
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
    /// Override the `out-dir` for the file
    ///
    /// Alternatively could simply use the absolute path
    #[arg(short, long, requires_all = &["port", "file"], value_parser = validate_dir)]
    out_dir: Option<PathBuf>,
    /// Override the `exit-script` that's run after writing to a file
    #[arg(long, requires_all = &["port", "file"], value_parser = is_script)]
    exit_script: Option<PathBuf>,
}

impl From<ConfigOverrides> for sericom_core::configs::ConfigOverride {
    fn from(overrides: ConfigOverrides) -> Self {
        sericom_core::configs::ConfigOverride {
            color: overrides.color,
            out_dir: overrides.out_dir,
            exit_script: overrides.exit_script,
        }
    }
}

async fn run_repl() -> miette::Result<()> {
    let conf = rustyline::Config::builder()
        .history_ignore_space(true)
        .build();
    let mut rl = rustyline::DefaultEditor::with_config(conf).expect("Failed to create REPL");
    let history_path = std::env::home_dir()
        .unwrap_or(PathBuf::from("./"))
        .join(".repl_history");

    if rl.load_history(&history_path).is_err() {
        println!("No previous history");
    }

    println!("Welcome to the sericom, type 'exit' to quit.");

    loop {
        let readline = rl.readline(">> ");
        match readline {
            Ok(line) => {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }

                rl.add_history_entry(line).ok();

                if line == "exit" {
                    break;
                }

                if line.contains('?') {
                    handle_help(line);
                    continue;
                }

                let args = format!("sericom {}", line);
                let cli = Cli::try_parse_from(args.split_whitespace());
                match cli {
                    Ok(cli) => {
                        if let Some(cmd) = cli.command {
                            handle_cmds(cmd).await?
                        }
                    }
                    Err(e) => eprintln!("{e}"),
                }
            }
            Err(_) => break,
        }
    }

    rl.save_history(&history_path).ok();
    Ok(())
}

fn handle_help(line: &str) {
    let mut cmd = Cli::command();
    let tokens: Vec<&str> = line.split_whitespace().collect();
    if tokens.is_empty() || tokens == ["?"] {
        cmd.print_help().unwrap();
        println!();
        return;
    }

    if let Some((sub, rest)) = tokens.split_first()
        && *rest == ["?"]
        && let Some(mut sub_cmd) = cmd
            .get_subcommands_mut()
            .find(|s| s.get_name() == *sub)
            .cloned()
    {
        sub_cmd.print_help().ok();
        println!();
        return;
    }

    println!("{RED}No help found for '{BOLD}{line}{UNBOLD}'.{RESET}");
}

#[tokio::main]
async fn main() -> miette::Result<()> {
    let cli = Cli::parse();

    if let Some(cmd) = cli.command {
        handle_cmds(cmd).await?
    } else {
        run_repl().await?
    }
    Ok(())
}

async fn handle_cmds(cmd: Commands) -> miette::Result<()> {
    match cmd {
        Commands::Connect {
            port,
            baud,
            config_override,
            file,
            debug,
        } => {
            let connection = open_connection(baud, &port)?;
            let overrides: sericom_core::configs::ConfigOverride = config_override.into();

            if let Some(Some(path)) = &file
                && path.is_dir()
            {
                return Err(miette::miette!(
                    "Could not create file at: '{}' because it is a directory.",
                    path.display()
                ));
            }
            initialize_config(overrides)?;
            // Need to hold the guard in `main`'s scope
            let _guard: Option<tracing_appender::non_blocking::WorkerGuard> = if debug {
                let config = get_config();
                let out_dir = config.defaults.debug_dir.as_path();
                init_tracing(out_dir, &port)?
            } else {
                None
            };
            interactive_session(connection, file, &port).await?;
            Ok(())
        }
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
            Ok(())
        }
        Commands::Ports => list_serial_ports(),
        Commands::Settings { baud, port } => get_settings(baud, &port),
    }
}

fn init_tracing<S>(
    out_dir: &Path,
    port: S,
) -> miette::Result<Option<tracing_appender::non_blocking::WorkerGuard>>
where
    S: AsRef<str> + Display + Into<PathBuf>,
{
    use sericom_core::compat_port_path;

    let path = compat_port_path!(out_dir, port, prefix = "trace");
    let file = std::fs::File::options()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&path)
        .into_diagnostic()
        .wrap_err_with(|| format!("Failed to create '{}'", path.display()))?;
    let (non_blocking, guard) = tracing_appender::non_blocking(file);
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
    Ok(Some(guard))
}
