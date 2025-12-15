use clap::{CommandFactory, Parser, Subcommand};
use miette::{Context as _, IntoDiagnostic};
use sericom_core::{
    cli::{color_parser, list_serial_ports, valid_baud_rate},
    path_utils::{is_script, validate_dir},
};
use std::path::PathBuf;

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
    #[command(
        visible_aliases = ["c", "con"],
        long_about = None,
        help_template = SUBCOMMAND_TEMPLATE,
        arg_required_else_help = true
    )]
    Connect {
        /// The path to a serial port
        ///
        /// For Linux/MacOS something like `/dev/ttyUSB0`, Windows `COM1`.
        port: PathBuf,
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
    #[command(
        visible_alias = "k",
        help_template = SUBCOMMAND_TEMPLATE,
        arg_required_else_help = true
    )]
    Kill {
        /// A space-delimited list of sessions to terminate
        ///
        /// For example: `kill 0 2 3` or `kill 0`
        session: Vec<sericom_core::session::SessionID>,
    },
    /// List helpful information
    #[command(
        visible_aliases = ["ls","l"],
        help_template = SUBCOMMAND_TEMPLATE,
        arg_required_else_help = true
    )]
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

pub async fn run_repl() -> miette::Result<()> {
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
                        Ok(false) => {
                            manager
                                .check_errors(|errs| {
                                    for (id, err) in errs {
                                        println!("[session {id}] {err:?}");
                                    }
                                })
                                .await;
                        }
                        Err(e) => println!("{e:?}"),
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
            #[allow(unused)]
            config_override,
            file,
            bg,
        } => {
            let res = match (file.as_ref(), bg) {
                (Some(_), bg) => manager.spawn(port, baud, file, bg).map(|_| false),
                (None, bg) => manager.spawn(port, baud, None, bg).map(|_| false),
            };

            // Allow initialization errors to propogate
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            manager
                .check_errors(|errs| {
                    for (id, err) in errs {
                        println!("[session {id}]:\n{err:?}");
                    }
                })
                .await;
            res
        }
        Commands::Kill { session } => {
            for id in session {
                manager.kill(id).await;
                tracing::debug!(target: "repl::kill", "session {id} closed");
            }
            Ok(false)
        }
        Commands::List { cmd } => match cmd {
            ListCmds::Ports => list_serial_ports()
                .wrap_err("Failed to list available ports. Platform unsupported")
                .map(|_| false),
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
