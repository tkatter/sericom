//! Sericom is a CLI tool for communicating with devices over a serial connection.
//!
//! Currently, Sericom runs similarily to another CLI tool called 'screen'. In the future,
//! Sericom plans to allow for users to create config files for customizing appearances
//! and defaults. Sericom also plans to allow the writing of custom scripts (similar to
//! expect scripts) that can be parsed and executed by Sericom. The intention of these
//! scripts is to be able to automate tasks that take place over a serial connection i.e.
//! configuration, resetting, getting statistics, etc.

use clap::{CommandFactory, Parser, Subcommand};
use miette::{Context, IntoDiagnostic};
use sericom_core::{
    cli::{color_parser, list_serial_ports, valid_baud_rate},
    configs::{get_config, initialize_config},
    path_utils::{is_script, validate_dir},
};
use std::path::{Path, PathBuf};

const RED: &str = "\x1b\x5b31m";
const BOLD: &str = "\x1b\x5b1m";
const UNBOLD: &str = "\x1b\x5b22m";
const DIM: &str = "\x1b\x5b2m";
const RESET: &str = "\x1b\x5b0m";

#[derive(Parser)]
#[command(name = "sericom", version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[allow(clippy::enum_variant_names)]
#[derive(Subcommand)]
enum Commands {
    /// Connect to a serial port
    #[command(alias = "c", alias = "con")]
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
        /// Start the session in the background (headless)
        #[arg(long)]
        bg: bool,
        /// Write debug output
        #[arg(short, long)]
        debug: bool,
    },
    /// List helpful information like valid baud rates, available serial ports, and sessions.
    #[command(alias = "ls", alias = "l")]
    List {
        #[command(subcommand)]
        cmd: ListCmds,
    },
    // TODO: Use this to print user settings like `git config --list`
    // Set {
    // Stuff to set settings
    // }
    // Settings {
    //     #[arg(short, long, value_parser = valid_baud_rate, default_value_t = 9600)]
    //     baud: u32,
    //     /// Path to the port to open
    //     #[arg(short, long)]
    //     port: String,
    // },
}

#[derive(Subcommand)]
enum ListCmds {
    /// List available serial ports.
    #[command(alias = "p")]
    Ports,
    /// List valid baud rates.
    #[command(alias = "b")]
    Bauds,
    #[command(alias = "s", alias = "sess")]
    Sessions,
    #[command(alias = "set")]
    Settings,
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

const CONFIG_OVERRIDE: sericom_core::configs::ConfigOverride =
    sericom_core::configs::ConfigOverride {
        color: None,
        out_dir: None,
        exit_script: None,
    };

#[tokio::main]
async fn main() -> miette::Result<()> {
    let mut manager = sericom_core::session::SessionManager::new();
    initialize_config(CONFIG_OVERRIDE)?;
    let _trace_guard: Option<tracing_appender::non_blocking::WorkerGuard> = {
        let config = get_config();
        let out_dir = config.defaults.debug_dir.as_path();
        init_tracing(out_dir, false)? // TODO: GET DEBUG CLI FLAG HERE
    };

    tokio::select! {
        result = run_repl(&mut manager) => result,
        () = shutdown_signal() => {
            tracing::info!("got shutdown signal");
            manager.graceful_shutdown().await
        }
    }
}

async fn run_repl(mut manager: &mut sericom_core::session::SessionManager) -> miette::Result<()> {
    let conf = rustyline::Config::builder()
        .history_ignore_space(true)
        .build();
    let mut rl = rustyline::DefaultEditor::with_config(conf)
        .into_diagnostic()
        .wrap_err("Failed to create REPL")?;
    let history_path = std::env::home_dir()
        .unwrap_or(PathBuf::from("./"))
        .join(".sericom_history");

    if rl.load_history(&history_path).is_err() {
        println!("{DIM}No previous history{RESET}");
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

                if line == "exit" || line == "q" || line == "quit" {
                    break;
                }

                if line.contains('?') {
                    handle_help(line);
                    continue;
                }

                let args = format!("sericom {}", line);
                let cli = Cli::try_parse_from(args.split_whitespace());
                match cli {
                    Ok(cli) => handle_cmds(cli.command, &mut manager).await?,
                    Err(e) => {
                        print!("{e}");
                    }
                }
            }
            Err(_) => break,
        }
    }

    rl.save_history(&history_path).ok();
    Ok(())
}

async fn handle_cmds(
    cmd: Commands,
    manager: &mut sericom_core::session::SessionManager,
) -> miette::Result<()> {
    let mut stdout = std::io::stdout();
    match cmd {
        Commands::Connect {
            port,
            baud,
            config_override,
            file,
            bg,
            debug,
        } => {
            manager
                .spawn(&port, baud)
                .wrap_err("Failed to set subscriber")?;
            // let connection = open_connection(baud, &port)?;
            // let overrides: sericom_core::configs::ConfigOverride = config_override.into();
            //
            // if let Some(Some(path)) = &file
            //     && path.is_dir()
            // {
            //     return Err(miette::miette!(
            //         "Could not create file at: '{}' because it is a directory.",
            //         path.display()
            //     ));
            // }
            // initialize_config(overrides)?;
            // // Need to hold the guard in `main`'s scope
            // let _guard: Option<tracing_appender::non_blocking::WorkerGuard> = if debug {
            //     let config = get_config();
            //     let out_dir = config.defaults.debug_dir.as_path();
            //     init_tracing(out_dir)?
            // } else {
            //     None
            // };
            // interactive_session(connection, file, &port).await?;
            Ok(())
        }
        Commands::List { cmd } => match cmd {
            ListCmds::Ports => list_serial_ports(),
            ListCmds::Bauds => {
                println!("Valid baud rates:");
                for baud in serial2_tokio::COMMON_BAUD_RATES {
                    println!("{baud}");
                }
                Ok(())
            }
            ListCmds::Sessions => {
                manager.list(&mut stdout);
                Ok(())
            }
            ListCmds::Settings => todo!(),
        },
        // Commands::Settings { baud, port } => get_settings(baud, &port),
    }
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

fn init_tracing(
    out_dir: &Path,
    _debug: bool,
) -> miette::Result<Option<tracing_appender::non_blocking::WorkerGuard>> {
    use sericom_core::compat_port_path;

    let path = compat_port_path!(out_dir);
    let file = std::fs::File::options()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&path)
        .into_diagnostic()
        .wrap_err_with(|| format!("Failed to create '{}'", path.display()))?;

    let (non_blocking, guard) = tracing_appender::non_blocking(file);
    let subscriber = tracing_subscriber::fmt()
        .with_max_level({
            #[cfg(debug_assertions)]
            {
                tracing::Level::TRACE
            }
            #[cfg(not(debug_assertions))]
            {
                if _debug {
                    tracing::Level::DEBUG
                } else {
                    tracing::Level::INFO
                }
            }
        })
        .with_writer(non_blocking)
        .with_line_number(false)
        .with_target(true)
        .finish();

    tracing::subscriber::set_global_default(subscriber)
        .into_diagnostic()
        .wrap_err("Failed to set subscriber")?;
    Ok(Some(guard))
}

async fn shutdown_signal() {
    use tokio::signal::{self, unix::SignalKind};

    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    let terminate = async {
        signal::unix::signal(SignalKind::terminate())
            .expect("Failed to install signal handler")
            .recv()
            .await
    };

    let quit = async {
        signal::unix::signal(SignalKind::quit())
            .expect("Failed to install signal handler")
            .recv()
            .await
    };

    tokio::select! {
        () = ctrl_c => {},
        _ = terminate => {},
        _ = quit => {},
    }
}
