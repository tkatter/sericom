//! Sericom is a CLI tool for communicating with devices over a serial connection.
//!
//! Currently, Sericom runs similarily to another CLI tool called 'screen'. In the future,
//! Sericom plans to allow for users to create config files for customizing appearances
//! and defaults. Sericom also plans to allow the writing of custom scripts (similar to
//! expect scripts) that can be parsed and executed by Sericom. The intention of these
//! scripts is to be able to automate tasks that take place over a serial connection i.e.
//! configuration, resetting, getting statistics, etc.

use clap::{CommandFactory, Parser, Subcommand};
use miette::{Context as _, IntoDiagnostic};
use sericom_core::{
    cli::{color_parser, list_serial_ports, valid_baud_rate},
    configs::{get_config, initialize_config},
    path_utils::{is_script, validate_dir},
};
use std::path::{Path, PathBuf};

const RED: &str = "\x1b\x5b31m";
const CYAN: &str = "\x1b\x5b36m";
const BOLD: &str = "\x1b\x5b1m";
// const UNBOLD: &str = "\x1b\x5b22m";
const DIM: &str = "\x1b\x5b2m";
const RESET: &str = "\x1b\x5b0m";
const PARSER_TEMPLATE: &str = "\
        {all-args}{after-help}\
";
const SUBCOMMAND_TEMPLATE: &str = "\
        {usage-heading}\n    {usage}\n\
        \n\
        {all-args}{after-help}\
";

#[derive(Parser)]
#[command(
    version,
    about,
    long_about = None,
    infer_subcommands = true,
    arg_required_else_help = true,
    help_template = PARSER_TEMPLATE,
    disable_colored_help = false,
    after_help = "For command specific help use `help [COMMAND]`.\n\
    To get help in the middle of a command you can use `?`.\n\
    For example: `list ?` will print the help text for the `list` command.
    ",
    multicall = true
)]
struct Repl {
    #[command(subcommand)]
    command: Commands,
}

#[allow(clippy::enum_variant_names)]
#[derive(Subcommand)]
enum Commands {
    /// Connect to a serial port
    #[command(visible_aliases = ["c", "con"], long_about = None, help_template = SUBCOMMAND_TEMPLATE)]
    Connect {
        /// The path to a serial port
        ///
        /// For Linux/MacOS something like `/dev/ttyUSB0`, Windows `COM1`.
        port: String,
        /// Baud rate for the serial connection
        #[arg(short, long, value_parser = valid_baud_rate, default_value_t = 9600)]
        baud: u32,
        /// Path to a file to write the session's output
        #[arg(short, long)]
        file: Option<Option<PathBuf>>,
        /// Start the session in the background (headless)
        #[arg(long)]
        bg: bool,
        #[clap(flatten)]
        config_override: ConfigOverrides,
    },
    /// Close a session
    #[command(visible_alias = "k", help_template = SUBCOMMAND_TEMPLATE)]
    Kill {
        session: Vec<sericom_core::session::SessionID>,
    },
    /// List helpful information
    #[command(visible_aliases = ["ls","l"], help_template = SUBCOMMAND_TEMPLATE)]
    List {
        #[command(subcommand)]
        cmd: ListCmds,
    },
    /// Exit the program
    #[command(visible_aliases = ["e", "q","quit"])]
    Exit,
    /// Print the version of sericom
    #[command(visible_short_flag_alias = 'V', visible_long_flag_alias = "version")]
    Version,
    // TODO: Use this to print user settings like `git config --list`
    // Set {
    //     /* Stuff to set settings */
    // }
}

#[derive(Subcommand)]
enum ListCmds {
    /// List the current sessions/connections
    #[command(visible_alias = "s")]
    Sessions,
    /// List available serial ports
    #[command(visible_alias = "p")]
    Ports,
    /// List valid baud rates
    #[command(visible_alias = "b")]
    Bauds,
    /// List the current configuration
    #[command(visible_alias = "conf")]
    Config,
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
    /// Override the `script` that's run after writing to a file
    #[arg(long, requires_all = &["port", "file"], value_parser = is_script)]
    script: Option<PathBuf>,
}

impl From<ConfigOverrides> for sericom_core::configs::ConfigOverride {
    fn from(overrides: ConfigOverrides) -> Self {
        sericom_core::configs::ConfigOverride {
            color: overrides.color,
            out_dir: overrides.out_dir,
            script: overrides.script,
        }
    }
}

#[tokio::main]
async fn main() -> miette::Result<()> {
    initialize_config(None)?;
    let _trace_guard: Option<tracing_appender::non_blocking::WorkerGuard> = {
        let config = get_config().unwrap();
        let dbg_dir = config.defaults.debug_dir.as_path();
        init_tracing(dbg_dir)?
    };

    run_repl().await
}

async fn run_repl() -> miette::Result<()> {
    let conf = rustyline::Config::builder()
        .history_ignore_space(true)
        .completion_show_all_if_ambiguous(true)
        .completion_type(rustyline::CompletionType::Circular)
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

    let mut manager = sericom_core::session::SessionManager::new();
    loop {
        let readline = rl.readline(">> ");
        match readline {
            Ok(line) => {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }

                rl.add_history_entry(line).ok();

                if line.contains('?') {
                    handle_help(line);
                    continue;
                }

                match Repl::try_parse_from(line.split_whitespace().collect::<Vec<&str>>()) {
                    Ok(cli) => match handle_cmds(cli.command, &mut manager).await {
                        Ok(true) => break,
                        Ok(false) => {}
                        Err(e) => {
                            println!("{RED}\u{1F5F4} {e}{RESET}");
                            for e in e.chain().skip(1) {
                                println!("  {RED}\u{2937}{RESET} {e}");
                            }

                            if let Some(help) = e.help() {
                                println!("\n{CYAN}{BOLD}help:{RESET} {help}\n");
                            }
                        }
                    },
                    Err(e) => e.print().expect("error printing clap error"),
                }
            }
            Err(rustyline::error::ReadlineError::Interrupted) => continue,
            Err(err) => {
                tracing::debug!(target: "repl", %err, "got unknown error from rustyline");
                break;
            }
        }
    }

    rl.save_history(&history_path).ok();
    manager.graceful_shutdown().await;
    Ok(())
}

async fn handle_cmds(
    cmd: Commands,
    manager: &mut sericom_core::session::SessionManager,
) -> miette::Result<bool> {
    match cmd {
        Commands::Connect {
            port,
            baud,
            config_override,
            file,
            bg,
        } => {
            if bg {
                return manager.spawn(&port, baud).map(|_| false);
            }

            let id = manager.spawn(&port, baud)?;
            if let Some(Some(path)) = &file
                && path.is_dir()
            {
                return Err(miette::miette!(
                    "Could not create file at: '{}' because it is a directory.",
                    path.display()
                ));
            }

            Ok(false)
        }
        Commands::Kill { session } => {
            for id in session {
                manager.kill(id).await;
                tracing::debug!(target: "repl::kill", "session {id} closed");
            }
            Ok(false)
        }
        Commands::List { cmd } => match cmd {
            ListCmds::Ports => list_serial_ports().map(|_| false),
            ListCmds::Bauds => {
                println!("Valid baud rates:");
                for baud in serial2_tokio::COMMON_BAUD_RATES {
                    println!("{baud}");
                }
                Ok(false)
            }
            ListCmds::Sessions => {
                manager.list(&mut std::io::stdout());
                Ok(false)
            }
            ListCmds::Config => todo!(),
        },
        Commands::Exit => Ok(true),
        Commands::Version => {
            println!("{}", Repl::command().render_long_version());
            Ok(false)
        }
    }
}

fn handle_help(line: &str) {
    let mut cmd = Repl::command();
    let tokens: Vec<&str> = line.split_whitespace().collect();
    if tokens == ["?"] || tokens.is_empty() {
        cmd.print_help().expect("help won't err");
        return;
    }

    if let Some((sub, rest)) = tokens.split_first()
        && *rest == ["?"]
        && let Some(mut sub_cmd) = cmd
            .get_subcommands_mut()
            .find(|s| s.get_name() == *sub)
            .cloned()
    {
        sub_cmd.print_help().expect("help won't err");
    }
}

fn init_tracing(
    dbg_dir: &Path,
) -> miette::Result<Option<tracing_appender::non_blocking::WorkerGuard>> {
    use sericom_core::compat_port_path;
    use tracing::{Level, level_filters::LevelFilter};
    use tracing_subscriber::EnvFilter;
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::util::SubscriberInitExt;
    use tracing_subscriber::{filter, fmt};

    let path = compat_port_path!(dbg_dir);
    let file = std::fs::File::options()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&path)
        .into_diagnostic()
        .wrap_err_with(|| format!("Failed to create '{}'", path.display()))?;

    let (non_blocking, guard) = tracing_appender::non_blocking(file);

    tracing_subscriber::registry()
        .with(
            fmt::layer()
                .with_writer(non_blocking)
                .with_line_number(false)
                .with_target(true),
        )
        .with(
            filter::Targets::new()
                .with_target("sericom", Level::TRACE)
                .with_target("sericom_core", Level::TRACE)
                .with_target("session", Level::TRACE)
                .with_target("repl", Level::TRACE)
                .with_target("exit_script", Level::TRACE)
                .with_default(Level::ERROR),
        )
        .with(
            EnvFilter::builder()
                .with_default_directive(
                    #[cfg(debug_assertions)]
                    LevelFilter::TRACE.into(),
                    #[cfg(not(debug_assertions))]
                    LevelFilter::INFO.into(),
                )
                .from_env_lossy(),
        )
        .init();

    Ok(Some(guard))
}
