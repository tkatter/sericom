//! This module holds the functions that are called from `sericom` when receiving
//! CLI commands/arguments.

// use crossterm::event;
use serial2_tokio::SerialPort;
use std::{
    // io::{self, Write},
    path::PathBuf,
};

use crate::map_miette;

/// Opens a serial `port` for communication with the specified `baud`.
///
/// Returns `Ok(SerialPort)` or errors if unable to set the baud rate or open the `port`.
pub fn open_connection(baud: u32, port: &PathBuf) -> miette::Result<SerialPort> {
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
        format!("Failed to open port '{}'", port.display()),
        help = "Is the port already open?\nTo see available ports, try `list ports`."
    )?;
    Ok(con)
}

/// Gets the settings for the `port` with the specified `baud`.
#[allow(clippy::many_single_char_names)]
pub fn get_settings(baud: u32, port: &PathBuf) -> miette::Result<()> {
    // https://www.contec.com/support/basic-knowledge/daq-control/serial-communicatin/
    let con = open_connection(baud, port)?;
    let settings = map_miette!(
        con.get_configuration(),
        format!("Failed to get settings for port '{}'", port.display())
    )?;
    let b = map_miette!(
        settings.get_baud_rate(),
        format!("Failed to get the baud rate for port '{}'", port.display())
    )?;
    let c = map_miette!(
        settings.get_char_size(),
        format!("Failed to get the char size for port '{}'", port.display())
    )?;
    let s = map_miette!(
        settings.get_stop_bits(),
        format!("Failed to get stop bits for port '{}'", port.display())
    )?;
    let p = map_miette!(
        settings.get_parity(),
        format!("Failed to get parity for port '{}'", port.display())
    )?;
    let f = map_miette!(
        settings.get_flow_control(),
        format!("Failed to get flow control for port '{}'", port.display())
    )?;

    let cts = map_miette!(
        con.read_cts(),
        format!("Failed to read CTS for port '{}'", port.display())
    )?;
    let dsr = map_miette!(
        con.read_dsr(),
        format!("Failed to read DSR for port '{}'", port.display())
    )?;
    let ri = map_miette!(
        con.read_ri(),
        format!("Failed to read RI for port '{}'", port.display())
    )?;
    let cd = map_miette!(
        con.read_cd(),
        format!("Failed to read CD for port '{}'", port.display())
    )?;

    println!("Baud rate: {b}");
    println!("Char size: {c}");
    println!("Stop bits: {s}");
    println!("Parity mechanism: {p}");
    println!("Flow control: {f}");
    println!("Clear To Send line: {cts}");
    println!("Data Set Ready line: {dsr}");
    println!("Ring Indicator line: {ri}");
    println!("Carrier Detect line: {cd}");

    Ok(())
}

/// Prints a list of available serial ports to stdout.
///
/// Ultimately a wrapper around [`SerialPort::available_ports()`] and may error
/// if it is called on an unsupported platform as per [`SerialPort::available_ports()`]s docs
pub fn list_serial_ports() -> miette::Result<()> {
    let ports = map_miette!(
        SerialPort::available_ports(),
        "Could not list available ports."
    )?;

    for path in ports {
        println!("{}", path.display());
    }
    Ok(())
}

/// Used as a [`value_parser`] for [`sericom`]s [`clap`] CLI
///
/// [`value_parser`]: https://docs.rs/clap/latest/clap/struct.Arg.html#method.value_parser
/// [`sericom`]: https://crates.io/crates/sericom
/// [`clap`]: https://crates.io/crates/clap
pub fn valid_baud_rate(s: &str) -> Result<u32, String> {
    let baud: u32 = s
        .parse()
        .map_err(|_| format!("`{s}` isn't a valid baud rate"))?;
    if serial2_tokio::COMMON_BAUD_RATES.contains(&baud) {
        Ok(baud)
    } else {
        Err(format!("'{baud}' is not a valid baud rate"))
    }
}

/// Used as a [`value_parser`] for [`sericom`]s [`clap`] CLI to parse args into a [`SeriColor`].
///
/// [`value_parser`]: https://docs.rs/clap/latest/clap/struct.Arg.html#method.value_parser
/// [`sericom`]: https://crates.io/crates/sericom
/// [`clap`]: https://crates.io/crates/clap
/// [`SeriColor`]: crate::configs::SeriColor
pub fn color_parser(input: &str) -> Result<crate::configs::SeriColor, String> {
    use crate::configs::{NORMALIZER, SeriColor};
    match SeriColor::parse_from_str(input, NORMALIZER) {
        Ok(c) => Ok(c),
        Err(valid_colors) => Err(format!("\n\nExpected one of: {}", valid_colors.join(", "))),
    }
}

/*
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

fn run_file_exit_script(file_path: PathBuf) {
    let config = crate::configs::get_config().unwrap(); //TODO: HANDLE
    let Some(script_path) = config.defaults.exit_script.as_ref() else {
        return;
    };
    let full_file_path = file_path
        .canonicalize()
        .expect("All error conditions have been checked");
    let cmd = create_platform_cmd(script_path, full_file_path);
    if let Ok(output) = cmd {
        tracing::debug!(
            target: "exit_script",
            "stdout: {}, stderr: {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
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
*/
