//! This module holds the functions that are called from `sericom` when receiving
//! CLI commands/arguments.

use crate::{
    compat_port_path,
    configs::get_config,
    create_recursive,
    debug::run_debug_output,
    map_miette,
    screen_buffer::UICommand,
    serial_actor::{
        SerialActor, SerialEvent, SerialMessage,
        tasks::{run_file_output, run_stdin_input, run_stdout_output},
    },
};
use crossterm::{
    cursor, event, execute,
    style::Stylize,
    terminal::{self, ClearType},
};
use miette::{Context, IntoDiagnostic};
use serial2_tokio::SerialPort;
use std::{
    io::{self, Write},
    path::PathBuf,
};
use tracing::{Level, trace};

/// Spawns all of the tasks responsible for maintaining an interactive terminal session.
pub async fn interactive_session(
    connection: SerialPort,
    file_path: Option<Option<PathBuf>>,
    debug: bool,
    port_name: &str,
) -> miette::Result<()> {
    let span = tracing::span!(Level::TRACE, "Interactive Session");
    let _enter = span.enter();
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
    let config = get_config();

    trace!("Creating channels");
    // Create channels
    let (command_tx, command_rx) = tokio::sync::mpsc::channel::<SerialMessage>(100);
    let (ui_tx, ui_rx) = tokio::sync::mpsc::channel::<UICommand>(100);
    let (broadcast_event_tx, _) = tokio::sync::broadcast::channel::<SerialEvent>(128);
    let stdout_rx = broadcast_event_tx.subscribe();

    // Create tasks
    let mut tasks = tokio::task::JoinSet::new();

    if let Some(maybe_path) = file_path {
        let default_out_dir = PathBuf::from(&config.defaults.out_dir);
        let file_path = match maybe_path {
            Some(path) => {
                // If given an absolute path - override the `default_out_dir`
                if path.is_absolute() {
                    let parent = path.parent().unwrap_or(&default_out_dir);
                    create_recursive!(parent);
                    path
                } else {
                    let joined_path = default_out_dir.join(&path);
                    let parent_path = joined_path.parent().expect("Does not have root");
                    create_recursive!(parent_path);
                    joined_path
                }
            }
            None => {
                let default_out_dir = PathBuf::from(&config.defaults.out_dir);
                compat_port_path!(default_out_dir, port_name)
            }
        };
        let file_rx = broadcast_event_tx.subscribe();
        tasks.spawn(async move {
            run_file_output(file_rx, file_path.clone()).await;
            run_file_exit_script(config, file_path);
        });
    };

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

/// Opens a serial `port` for communication with the specified `baud`.
///
/// Returns `Ok(SerialPort)` or errors if unable to set the baud rate or open the `port`.
pub fn open_connection(baud: u32, port: &str) -> miette::Result<SerialPort> {
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
        format!(
            "{} {} [OPTIONS] [PORT] [COMMAND]",
            "USAGE:".bold().underlined(),
            "sericom".bold()
        ),
        help = format!(
            "To see available ports, try `{}`.",
            "sericom list-ports".bold().cyan()
        )
    )?;
    Ok(con)
}

/// Gets the settings for the `port` with the specified `baud`.
pub fn get_settings(baud: u32, port: &str) -> miette::Result<()> {
    // https://www.contec.com/support/basic-knowledge/daq-control/serial-communicatin/
    let mut stdout = io::stdout();
    let con = open_connection(baud, port)?;
    let settings = map_miette!(
        con.get_configuration(),
        format!("Failed to get settings for port '{}'", port),
        format!(
            "{} {} [OPTIONS] {} <PORT>",
            "USAGE:".bold().underlined(),
            "sericom list-settings".bold(),
            "--port".bold()
        )
    )?;
    let b = map_miette!(
        settings.get_baud_rate(),
        format!("Failed to get the baud rate for port '{}'", port),
        format!(
            "{} {} [OPTIONS] {} <PORT>",
            "USAGE:".bold().underlined(),
            "sericom list-settings".bold(),
            "--port".bold()
        )
    )?;
    let c = map_miette!(
        settings.get_char_size(),
        format!("Failed to get the char size for port '{}'", port),
        format!(
            "{} {} [OPTIONS] {} <PORT>",
            "USAGE:".bold().underlined(),
            "sericom list-settings".bold(),
            "--port".bold()
        )
    )?;
    let s = map_miette!(
        settings.get_stop_bits(),
        format!("Failed to get stop bits for port '{}'", port),
        format!(
            "{} {} [OPTIONS] {} <PORT>",
            "USAGE:".bold().underlined(),
            "sericom list-settings".bold(),
            "--port".bold()
        )
    )?;
    let p = map_miette!(
        settings.get_parity(),
        format!("Failed to get parity for port '{}'", port),
        format!(
            "{} {} [OPTIONS] {} <PORT>",
            "USAGE:".bold().underlined(),
            "sericom list-settings".bold(),
            "--port".bold()
        )
    )?;
    let f = map_miette!(
        settings.get_flow_control(),
        format!("Failed to get flow control for port '{}'", port),
        format!(
            "{} {} [OPTIONS] {} <PORT>",
            "USAGE:".bold().underlined(),
            "sericom list-settings".bold(),
            "--port".bold()
        )
    )?;

    let cts = map_miette!(
        con.read_cts(),
        format!("Failed to read CTS for port '{}'", port),
        format!(
            "{} {} [OPTIONS] {} <PORT>",
            "USAGE:".bold().underlined(),
            "sericom list-settings".bold(),
            "--port".bold()
        )
    )?;
    let dsr = map_miette!(
        con.read_dsr(),
        format!("Failed to read DSR for port '{}'", port),
        format!(
            "{} {} [OPTIONS] {} <PORT>",
            "USAGE:".bold().underlined(),
            "sericom list-settings".bold(),
            "--port".bold()
        )
    )?;
    let ri = map_miette!(
        con.read_ri(),
        format!("Failed to read RI for port '{}'", port),
        format!(
            "{} {} [OPTIONS] {} <PORT>",
            "USAGE:".bold().underlined(),
            "sericom list-settings".bold(),
            "--port".bold()
        )
    )?;
    let cd = map_miette!(
        con.read_cd(),
        format!("Failed to read CD for port '{}'", port),
        format!(
            "{} {} [OPTIONS] {} <PORT>",
            "USAGE:".bold().underlined(),
            "sericom list-settings".bold(),
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

/// Prints a list of available serial ports to stdout.
///
/// Ultimately a wrapper around [`SerialPort::available_ports()`] and may error
/// if it is called on an unsupported platform as per [`SerialPort::available_ports()]s docs
pub fn list_serial_ports() -> miette::Result<()> {
    let mut stdout = io::stdout();
    let ports = map_miette!(
        SerialPort::available_ports(),
        "Could not list available ports."
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

/// Used as a [`value_parser`](https://docs.rs/clap/latest/clap/struct.Arg.html#method.value_parser) for [`sericom`](https://crates.io/crates/sericom)s [`clap`](https://docs.rs/clap) CLI
/// struct to validate and parse args into a baud rate.
pub fn valid_baud_rate(s: &str) -> Result<u32, String> {
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

/// Used as a [`value_parser`](https://docs.rs/clap/latest/clap/struct.Arg.html#method.value_parser) for [`sericom`](https://crates.io/crates/sericom)s [`clap`](https://docs.rs/clap) CLI
/// struct to validate and parse args into a [`SeriColor`][`crate::configs::SeriColor`].
pub fn color_parser(input: &str) -> Result<crate::configs::SeriColor, String> {
    use crate::configs::{NORMALIZER, SeriColor};
    match SeriColor::parse_from_str(input, NORMALIZER) {
        Ok(c) => Ok(c),
        Err(valid_colors) => Err(format!("\n\nExpected one of: {}", valid_colors.join(", "))),
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

fn run_file_exit_script(config: &'static crate::configs::Config, file_path: PathBuf) {
    let span = tracing::span!(Level::DEBUG, "Exit script");
    let _enter = span.enter();

    let Some(script_path) = config.defaults.exit_script.as_ref() else {
        return;
    };
    let full_file_path = file_path
        .canonicalize()
        .expect("All error conditions have been checked");
    let cmd = create_platform_cmd(script_path, full_file_path);
    if let Ok(output) = cmd {
        let msg = format!(
            "stdout: {}, stderr: {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        tracing::debug!(msg);
    }
}

fn create_platform_cmd(
    script: &std::path::Path,
    file_path: std::path::PathBuf,
) -> Result<std::process::Output, io::Error> {
    use std::process::Command;

    #[cfg(unix)]
    {
        Command::new(script)
            .env("SERICOM_OUT_FILE", file_path)
            .output()
    }

    #[cfg(windows)]
    {
        let ext = script.extension().expect("Validated in initialization");
        match ext
            .to_ascii_lowercase()
            .to_str()
            .expect("Converted to ascii")
        {
            "ps1" => Command::new("powershell.exe")
                .arg("-File")
                .arg(script)
                .env("SERICOM_OUT_FILE", file_path)
                .output(),
            _ => Command::new("cmd.exe")
                .arg("/C")
                .arg(script)
                .env("SERICOM_OUT_FILE", file_path)
                .output(),
        }
    }
}
