use std::{
    fs::File,
    io::{self, Write},
};

use clap::{CommandFactory, Parser, Subcommand};
use crossterm::{cursor, event::{self, Event, KeyCode, KeyEvent, KeyModifiers}, execute, style::Print, terminal::{self, ClearType} };
use serial2_tokio::SerialPort;

#[derive(Parser)]
#[command(name = "netcon", version, about, long_about = None)]
#[command(next_line_help = true)]
#[command(propagate_version = true)]
struct Cli {
    /// The path to a serial port.
    ///
    /// For Linux/MacOS something like '/dev/tty1', Windows 'COM1'.
    /// To see available ports, use `netcon list-ports`.
    port: Option<String>,
    #[arg(short, long, value_parser = valid_baud_rate, default_value_t = 9600)]
    baud: u32,
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Lists all valid baud rates
    ListBauds,
    /// Lists all available serial ports
    ListPorts {
        /// [DEFAULT] - Streams the serial output to stdout
        #[arg(short, long, default_value_t = true)]
        stream: bool,
        /// Writes the serial output to the specified file
        #[arg(short, long)]
        file: Option<String>,
    },
    /// Gets the settings for a serial port
    ListSettings {
        /// Specify the baud rate for the serial connection - REQUIRED IF '--keep-settings' NOT
        /// PRESENT
        #[arg(short, long, value_parser = valid_baud_rate, required_unless_present = "keep_settings")]
        baud: Option<u32>,
        /// Path to the port to open
        #[arg(short, long)]
        port: String,
        /// Keeps the existing serial port configuration
        #[arg(short, long)]
        keep_settings: bool,
        /// [DEFAULT] - Streams the serial output to stdout
        #[arg(short, long, default_value_t = true)]
        stream: bool,
        /// Writes the serial output to the specified file
        #[arg(short, long)]
        file: Option<String>,
    },
}

const UTF_TAB: &str = "\u{0009}";
const UTF_BKSP: &str = "\u{0008}";
const UTF_DEL: &str = "\u{007F}";
const UTF_ESC: &str = "\u{001B}";
const UTF_CTRL_C: &str = "\u{001B}\u{0043}";
const UTF_UP_KEY: &str = "\u{001B}\u{005B}\u{0041}";
const UTF_DOWN_KEY: &str = "\u{001B}\u{005B}\u{0042}";
const UTF_LEFT_KEY: &str = "\u{001B}\u{005B}\u{0044}";
const UTF_RIGHT_KEY: &str = "\u{001B}\u{005B}\u{0043}";

#[tokio::main]
async fn main() -> io::Result<()> {
    let cli = Cli::parse();

    if cli.port.is_none() && cli.command.is_none() {
            let mut cmd = Cli::command();
            cmd.error(
                clap::error::ErrorKind::MissingRequiredArgument,
                "Missing either PORT or COMMAND."
            ).exit();
    }

    if cli.port.is_some() && cli.command.is_some() {
            let mut cmd = Cli::command();
            cmd.error(
                clap::error::ErrorKind::ArgumentConflict,
                "Must specify either PORT or SUBCOMMAND, not both."
            ).exit();
    }

    if let Some(port) = cli.port {
        interactive_session(cli.baud, &port).await?;
    } else if let Some(cmd) = cli.command {
        match cmd {
            Commands::ListBauds => {
                let mut handle = io::stdout().lock();
                write!(handle, "Valid baud rates:\r\n")?;
                for baud in serial2_tokio::COMMON_BAUD_RATES {
                    write!(handle, "{baud}\r\n")?;
                }
            }
            Commands::ListPorts { stream, file } => {
                let handle = io::stdout().lock();
                if let Some(file) = file {
                    let path = std::path::Path::new(&file);
                    let mut file_handle = File::options().append(true).create(true).open(path)?;
                    if path.metadata()?.len() == 0 {
                        write!(file_handle, "UTC: {}\r\n", chrono::Utc::now())?;
                    } else {
                        write!(file_handle, "\r\nUTC: {}\r\n", chrono::Utc::now())?;
                    }
                    list_serial_ports(Box::new(file_handle))?
                } else if stream {
                    list_serial_ports(Box::new(handle))?
                }
            }
            Commands::ListSettings { baud, port, keep_settings, stream, file } => {
                let handle = io::stdout().lock();
                if let Some(file) = file {
                    let path = std::path::Path::new(&file);
                    let mut file_handle = File::options().append(true).create(true).open(path)?;
                    if path.metadata()?.len() == 0 {
                        write!(file_handle, "TIMESTAMP: {}\r\nPORT: {port}\r\n", chrono::Utc::now())?;
                    } else {
                        write!(file_handle, "\r\nTIMESTAMP: {}\r\nPORT: {port}\r\n", chrono::Utc::now())?;
                    }
                    get_settings(Box::new(file_handle), baud, &port, keep_settings)?;
                } else if stream {
                    get_settings(Box::new(handle), baud, &port, keep_settings)?;
                }
            }
        }
    }
    Ok(())
}

async fn interactive_session(baud: u32, port: &str) -> io::Result<()> {
    use tokio::{sync::{broadcast, Mutex}, time::{timeout, Duration}};
    use std::sync::Arc;
    let mut stdout = io::stdout();
    execute!(stdout, terminal::EnterAlternateScreen)?;
    terminal::enable_raw_mode()?;
    execute!(stdout, terminal::Clear(ClearType::All), cursor::MoveTo(0,0)).ok();

    // Sends a kill signal to all tokio processes
    let (shutdown_tx, _) = broadcast::channel::<()>(1);
    let (tx, mut rx) = tokio::sync::mpsc::channel::<Vec<u8>>(100);
    let con = Arc::new(Mutex::new(temp_open_port(baud, port)?));

    // Reads the incoming serial data from the connection and sends it
    // to the `write_handle` via `mpsc::channel` to be written to stdout
    let read_con = Arc::clone(&con);
    let mut read_shutdown_rx = shutdown_tx.subscribe();
    let read_handle = tokio::spawn(async move {
        let mut buffer = vec![0u8; 1024];
        loop {
            tokio::select! {
                _ = read_shutdown_rx.recv() => {
                    break;
                }
                read_result = async {
                    let connection = read_con.lock().await;
                    timeout(Duration::from_millis(100), connection.read(&mut buffer)).await 
                } => {
                    match read_result {
                        Ok(Ok(0)) => {
                            eprintln!("Serial connection closed");
                            break;
                        }
                        Ok(Ok(n)) => {
                            let data = buffer[..n].to_vec();
                            if tx.send(data).await.is_err() {
                                break;
                            }
                        }
                        Ok(Err(e)) => {
                            eprintln!("Read error: {e}");
                            break;
                        }
                        Err(_) => continue,
                    }
                }
            }
        }
    });

    // Writes incoming serial data from the connection to stdout via `mpsc::channel`
    let mut print_shutdown_rx = shutdown_tx.subscribe();
    let print_stdout_handle = tokio::spawn(async move {
        while let Some(data) = rx.recv().await {
            tokio::select! {
                _ = print_shutdown_rx.recv() => {
                    break;
                }
                _ = tokio::task::spawn_blocking(move || {
                    let mut stdout = io::stdout();
                    execute!(stdout, crossterm::style::SetColors(crossterm::style::Colors::new(crossterm::style::Color::Green, crossterm::style::Color::Black)), Print(String::from_utf8_lossy(&data).to_string())).ok();
                    stdout.flush().ok()
                }) => {}
            }
        }
    });

    // TODO: Implement asynchronous logging to a file?
    // thinking that i need to read data to a buffer x amount of bytes
    // to efficiently scan the buffer for certain words like `Error` or `serial`
    // also need to look into a proper channel for the serial connection data streaming
    // currently using mpsc but i only have one producer and will be having multiple consumers...
    // let log_file = tokio::spawn(async move {
    //     while let Some(data) = rx.recv().await {
    //     }
    // });

    // Reads data from stdin and sends it to the serial connection
    let write_con = Arc::clone(&con);
    let mut write_shutdown_rx = shutdown_tx.subscribe();
    let write_handle = tokio::spawn(async move {
        let (stdin_tx, mut stdin_rx) = tokio::sync::mpsc::channel::<String>(10);

        // Spawn blocking thread for stdin - use std::thread::spawn, not spawn_blocking
        std::thread::spawn(move || {
            let mut stdout = io::stdout();
            loop {
                match event::read() {
                    Ok(Event::Key(KeyEvent { code, modifiers, kind, .. })) => {
                        if kind != crossterm::event::KeyEventKind::Press {
                            continue;
                        }

                        match (code, modifiers) {
                            (KeyCode::Char('l'), KeyModifiers::CONTROL) => {
                                execute!(stdout, terminal::Clear(ClearType::All), cursor::MoveTo(0,0)).ok();
                            }
                            (KeyCode::Char('q'), KeyModifiers::CONTROL) => {
                                let _ = shutdown_tx.send(());
                                break;
                            }
                            (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                                if stdin_tx.blocking_send(UTF_CTRL_C.to_string()).is_err() {
                                    break;
                                }
                            }
                            (KeyCode::Tab, _) => {
                                if stdin_tx.blocking_send(UTF_TAB.to_string()).is_err() {
                                    break;
                                }
                            }
                            (KeyCode::Delete, _) => {
                                if stdin_tx.blocking_send(UTF_DEL.to_string()).is_err() {
                                    break;
                                }
                            }
                            (KeyCode::Up, _) => {
                                if stdin_tx.blocking_send(UTF_UP_KEY.to_string()).is_err() {
                                    break;
                                }
                            }
                            (KeyCode::Down, _) => {
                                if stdin_tx.blocking_send(UTF_DOWN_KEY.to_string()).is_err() {
                                    break;
                                }
                            }
                            (KeyCode::Left, _) => {
                                if stdin_tx.blocking_send(UTF_LEFT_KEY.to_string()).is_err() {
                                    break;
                                }
                            }
                            (KeyCode::Right, _) => {
                                if stdin_tx.blocking_send(UTF_RIGHT_KEY.to_string()).is_err() {
                                    break;
                                }
                            }
                            (KeyCode::Enter, _) => {
                                if stdin_tx.blocking_send('\r'.to_string()).is_err() {
                                    break;
                                }
                            }
                            (KeyCode::Backspace, _) => {
                                if stdin_tx.blocking_send(UTF_BKSP.to_string()).is_err() {
                                    break;
                                };
                            }
                            (KeyCode::Esc, _) => {
                                if stdin_tx.blocking_send(UTF_ESC.to_string()).is_err() {
                                    break;
                                };
                            }
                            (KeyCode::Char(c), _) => {
                                if stdin_tx.blocking_send(c.to_string()).is_err() {
                                    break;
                                };
                            }
                            _ => {}
                        }
                        stdout.flush().ok();
                    }
                    Ok(_) => {} // Ignore other events
                    Err(_) => break,
                }
            }
        });

        // Async task receives from channel and writes to serial
        while let Some(data) = stdin_rx.recv().await {
            tokio::select! {
                _ = write_shutdown_rx.recv() => {
                    break;
                }
                result = async {
                    let connection = write_con.lock().await;
                    connection.write_all(data.as_bytes()).await
                } => {
                    if let Err(e) = result {
                        eprintln!("Write error: {e}");
                        break;
                    }
                }
            };
        }
    });
    let result = tokio::try_join!(read_handle, print_stdout_handle, write_handle);
    ensure_terminal_cleanup(stdout);
    result?;
    Ok(())
}

fn temp_open_port(baud: u32, port: &str) -> io::Result<SerialPort> {
    let settings = |mut s: serial2_tokio::Settings| -> std::io::Result<serial2_tokio::Settings> {
        s.set_raw();
        s.set_baud_rate(baud)?;
        s.set_char_size(serial2_tokio::CharSize::Bits8);
        s.set_stop_bits(serial2_tokio::StopBits::One);
        s.set_parity(serial2_tokio::Parity::None);
        s.set_flow_control(serial2_tokio::FlowControl::None);
        Ok(s)
    };
    let con = SerialPort::open(port, settings)?;
    Ok(con)
}

// TODO: Consolidate this funciton to the `temp_open_port` function
fn open_port(baud: Option<u32>, port: &str, keep_settings: bool) -> io::Result<SerialPort> {
    let con = if keep_settings {
        SerialPort::open(port, serial2_tokio::KeepSettings)?
    } else if let Some(baud) = baud {
        let settings = |mut s: serial2_tokio::Settings| -> std::io::Result<serial2_tokio::Settings> {
            s.set_raw();
            s.set_baud_rate(baud)?;
            s.set_char_size(serial2_tokio::CharSize::Bits8);
            s.set_stop_bits(serial2_tokio::StopBits::One);
            s.set_parity(serial2_tokio::Parity::None);
            s.set_flow_control(serial2_tokio::FlowControl::None);
            Ok(s)
        };
        SerialPort::open(port, settings)?
    } else {
        unreachable!()
    };
    Ok(con)
}

fn get_settings(mut handle: Box<dyn io::Write>, baud: Option<u32>, port: &str, keep_settings: bool) -> Result<(), io::Error> {
    // https://www.contec.com/support/basic-knowledge/daq-control/serial-communicatin/
    let con = open_port(baud, port, keep_settings)?;
    let settings = con.get_configuration()?;

    let b = settings.get_baud_rate()?;
    let c = settings.get_char_size()?;
    let s = settings.get_stop_bits()?;
    let p = settings.get_parity()?;
    let f = settings.get_flow_control()?;

    let cts = con.read_cts()?;
    let dsr = con.read_dsr()?;
    let ri = con.read_ri()?;
    let cd = con.read_cd()?;

    write!(handle, "Baud rate: {b}\r\n")?;
    write!(handle, "Char size: {c}\r\n")?;
    write!(handle, "Stop bits: {s}\r\n")?;
    write!(handle, "Parity mechanism: {p}\r\n")?;
    write!(handle, "Flow control: {f}\r\n")?;
    write!(handle, "Clear To Send line: {cts}\r\n")?;
    write!(handle, "Data Set Ready line: {dsr}\r\n")?;
    write!(handle, "Ring Indicator line: {ri}\r\n")?;
    write!(handle, "Carrier Detect line: {cd}\r\n")?;

    Ok(())
}

fn list_serial_ports(mut handle: Box<dyn io::Write>) -> Result<(), io::Error> {
    let ports = SerialPort::available_ports()?;
    for path in ports {
        if let Some(path) = path.to_str() {
            let line = [path, "\r\n"].concat();
            handle.write(line.as_bytes())?
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
    use crossterm::{cursor::Show, execute, terminal::{disable_raw_mode, LeaveAlternateScreen}};
    let _ = disable_raw_mode();
    let _ = execute!(stdout, LeaveAlternateScreen, Show);
    let _ = stdout.flush();
}
