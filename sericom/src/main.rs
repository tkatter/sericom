//! Sericom is a CLI tool for communicating with devices over a serial connection.
//!
//! Sericom runs on Linux and Windows (untested on Mac). This crate is not intended
//! to be used by other crates - its intention is to be ran as an executable.
//! That being said, if there is interest in using Sericom within other crates, or maybe
//! certain functionality of Sericom within other crates, feel free to open a Github Issue.
//!
//! Currently, Sericom runs similarily to another CLI tool called 'screen'. In the future,
//! Sericom plans to allow for users to create config files for customizing appearances
//! and defaults. Sericom also plans to allow the writing of custom scripts (similar to
//! expect scripts) that can be parsed and executed by Sericom. The intention of these
//! scripts is to be able to automate tasks that take place over a serial connection i.e.
//! configuration, resetting, getting statistics, etc.

pub use clap::{CommandFactory, Parser, Subcommand};
pub use crossterm::{
    cursor, event, execute,
    style::Stylize,
    terminal::{self, ClearType},
};
use miette::{Context, IntoDiagnostic};
use serial2_tokio::SerialPort;
use sericom_core::{
    configs::{get_config, initialize_config},
    create_recursive,
    debug::run_debug_output,
    map_miette,
    screen_buffer::UICommand,
    serial_actor::{
        SerialActor, SerialEvent, SerialMessage,
        tasks::{run_file_output, run_stdin_input, run_stdout_output},
    },
};
use std::{
    io::{self, Write},
    path::PathBuf,
};

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
    let cli = Cli::parse();

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
                list_serial_ports()?;
            }
            Commands::ListSettings { baud, port } => {
                get_settings(baud, &port)?;
            }
        }
    }
    Ok(())
}

async fn interactive_session(
    connection: SerialPort,
    file: Option<String>,
    debug: bool,
    port_name: &str,
) -> miette::Result<()> {
    // Setup terminal
    let mut stdout = io::stdout();
    terminal::enable_raw_mode()
        .into_diagnostic()
        .wrap_err("Failed to enable raw mode.".red())?;
    execute!(
        stdout,
        terminal::EnterAlternateScreen,
        terminal::SetTitle(port_name),
        terminal::Clear(ClearType::All),
        event::EnableBracketedPaste,
        event::EnableMouseCapture,
        cursor::MoveTo(0, 0)
    )
    .into_diagnostic()
    .wrap_err("Failed to setup the terminal.".red())?;

    // Create channels
    let (command_tx, command_rx) = tokio::sync::mpsc::channel::<SerialMessage>(100);
    let (ui_tx, ui_rx) = tokio::sync::mpsc::channel::<UICommand>(100);
    let (broadcast_event_tx, _) = tokio::sync::broadcast::channel::<SerialEvent>(128);
    let stdout_rx = broadcast_event_tx.subscribe();

    // Create tasks
    let mut tasks = tokio::task::JoinSet::new();

    if let Some(path) = file {
        let config = get_config();
        let default_out_dir = PathBuf::from(&config.defaults.out_dir);
        let input_path = PathBuf::from(path);

        let file_path = if input_path.is_absolute() {
            let parent = input_path.parent().unwrap_or(&default_out_dir);
            create_recursive!(parent);
            input_path
        } else {
            let joined_path = default_out_dir.join(input_path);
            let parent_path = joined_path.parent().expect("Does not have root");
            create_recursive!(parent_path);
            joined_path
        };

        let file_rx = broadcast_event_tx.subscribe();
        tasks.spawn(run_file_output(file_rx, file_path));
    }

    if debug {
        let debug_rx = broadcast_event_tx.subscribe();
        tasks.spawn(run_debug_output(debug_rx));
    }

    let actor = SerialActor::new(connection, command_rx, broadcast_event_tx);
    tasks.spawn(actor.run());

    tasks.spawn(run_stdout_output(stdout_rx, ui_rx));
    tasks.spawn(run_stdin_input(command_tx, ui_tx));

    tasks.join_all().await;
    ensure_terminal_cleanup(stdout);
    Ok(())
}

fn open_connection(baud: u32, port: &str) -> miette::Result<SerialPort> {
    let settings = |mut s: serial2_tokio::Settings| -> std::io::Result<serial2_tokio::Settings> {
        s.set_raw();
        s.set_baud_rate(baud)?;
        s.set_char_size(serial2_tokio::CharSize::Bits8);
        s.set_stop_bits(serial2_tokio::StopBits::One);
        s.set_parity(serial2_tokio::Parity::None);
        s.set_flow_control(serial2_tokio::FlowControl::None);
        Ok(s)
    };
    let con = map_miette!(
        SerialPort::open(port, settings),
        format!("Failed to open port '{}'", port),
        "[OPTIONS] [PORT] [COMMAND]",
        help = format!(
            "To see available ports, try `{}`.",
            "sericom list-ports".bold().cyan()
        )
    )?;
    Ok(con)
}

fn get_settings(baud: u32, port: &str) -> miette::Result<()> {
    // https://www.contec.com/support/basic-knowledge/daq-control/serial-communicatin/
    let mut stdout = io::stdout();
    let con = open_connection(baud, port)?;
    let settings = map_miette!(
        con.get_configuration(),
        format!("Failed to get settings for port '{}'", port),
        format!(
            "{} [OPTIONS] {} <PORT>",
            "list-settings".bold(),
            "--port".bold()
        )
    )?;
    let b = map_miette!(
        settings.get_baud_rate(),
        format!("Failed to get the baud rate for port '{}'", port),
        format!(
            "{} [OPTIONS] {} <PORT>",
            "list-settings".bold(),
            "--port".bold()
        )
    )?;
    let c = map_miette!(
        settings.get_char_size(),
        format!("Failed to get the char size for port '{}'", port),
        format!(
            "{} [OPTIONS] {} <PORT>",
            "list-settings".bold(),
            "--port".bold()
        )
    )?;
    let s = map_miette!(
        settings.get_stop_bits(),
        format!("Failed to get stop bits for port '{}'", port),
        format!(
            "{} [OPTIONS] {} <PORT>",
            "list-settings".bold(),
            "--port".bold()
        )
    )?;
    let p = map_miette!(
        settings.get_parity(),
        format!("Failed to get parity for port '{}'", port),
        format!(
            "{} [OPTIONS] {} <PORT>",
            "list-settings".bold(),
            "--port".bold()
        )
    )?;
    let f = map_miette!(
        settings.get_flow_control(),
        format!("Failed to get flow control for port '{}'", port),
        format!(
            "{} [OPTIONS] {} <PORT>",
            "list-settings".bold(),
            "--port".bold()
        )
    )?;

    let cts = map_miette!(
        con.read_cts(),
        format!("Failed to read CTS for port '{}'", port),
        format!(
            "{} [OPTIONS] {} <PORT>",
            "list-settings".bold(),
            "--port".bold()
        )
    )?;
    let dsr = map_miette!(
        con.read_dsr(),
        format!("Failed to read DSR for port '{}'", port),
        format!(
            "{} [OPTIONS] {} <PORT>",
            "list-settings".bold(),
            "--port".bold()
        )
    )?;
    let ri = map_miette!(
        con.read_ri(),
        format!("Failed to read RI for port '{}'", port),
        format!(
            "{} [OPTIONS] {} <PORT>",
            "list-settings".bold(),
            "--port".bold()
        )
    )?;
    let cd = map_miette!(
        con.read_cd(),
        format!("Failed to read CD for port '{}'", port),
        format!(
            "{} [OPTIONS] {} <PORT>",
            "list-settings".bold(),
            "--port".bold()
        )
    )?;

    write!(stdout, "Baud rate: {b}\r\n")
        .into_diagnostic()
        .wrap_err("Failed to write to stdout.".red())?;
    write!(stdout, "Char size: {c}\r\n")
        .into_diagnostic()
        .wrap_err("Failed to write to stdout.".red())?;
    write!(stdout, "Stop bits: {s}\r\n")
        .into_diagnostic()
        .wrap_err("Failed to write to stdout.".red())?;
    write!(stdout, "Parity mechanism: {p}\r\n")
        .into_diagnostic()
        .wrap_err("Failed to write to stdout.".red())?;
    write!(stdout, "Flow control: {f}\r\n")
        .into_diagnostic()
        .wrap_err("Failed to write to stdout.".red())?;
    write!(stdout, "Clear To Send line: {cts}\r\n")
        .into_diagnostic()
        .wrap_err("Failed to write to stdout.".red())?;
    write!(stdout, "Data Set Ready line: {dsr}\r\n")
        .into_diagnostic()
        .wrap_err("Failed to write to stdout.".red())?;
    write!(stdout, "Ring Indicator line: {ri}\r\n")
        .into_diagnostic()
        .wrap_err("Failed to write to stdout.".red())?;
    write!(stdout, "Carrier Detect line: {cd}\r\n")
        .into_diagnostic()
        .wrap_err("Failed to write to stdout.".red())?;

    Ok(())
}

fn list_serial_ports() -> miette::Result<()> {
    let mut stdout = io::stdout();
    let ports = map_miette!(
        SerialPort::available_ports(),
        "Could not list available ports.",
        "list-ports".bold()
    )?;
    for path in ports {
        if let Some(path) = path.to_str() {
            let line = [path, "\r\n"].concat();
            stdout
                .write(line.as_bytes())
                .into_diagnostic()
                .wrap_err("Failed to write to stdout.".red())?
        } else {
            continue;
        };
    }
    Ok(())
}

fn valid_baud_rate(s: &str) -> Result<u32, String> {
    let baud: u32 = s
        .parse()
        .map_err(|_| format!("`{s}` isn't a valid baud rate"))?;
    if serial2_tokio::COMMON_BAUD_RATES.contains(&baud) {
        Ok(baud)
    } else {
        Err(format!(
            "'{}' is not a valid baud rate; valid baud rates include {:?}",
            baud,
            serial2_tokio::COMMON_BAUD_RATES
        ))
    }
}

fn ensure_terminal_cleanup(mut stdout: io::Stdout) {
    use crossterm::{
        cursor::Show,
        execute,
        terminal::{LeaveAlternateScreen, disable_raw_mode},
    };
    let _ = execute!(
        stdout,
        event::DisableMouseCapture,
        event::DisableBracketedPaste,
        LeaveAlternateScreen,
        Show
    );
    let _ = disable_raw_mode();
    let _ = stdout.flush();
}
