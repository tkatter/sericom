use std::{
    fs::File,
    io::{self, Write},
};

use clap::{Parser, Subcommand};
use crossterm::{cursor, event::{self, Event, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind}, execute, style::Print, terminal::{self, ClearType} };
use serial2_tokio::SerialPort;

#[derive(Parser)]
#[command(name = "SerialTool", version, about, long_about = None)]
#[command(next_line_help = true)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
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
    /// Opens a port and reads the recieved data
    PortSesh {
        /// Specify the baud rate for the serial connection - REQUIRED IF '--keep-settings' NOT
        /// PRESENT
        #[arg(short, long, value_parser = valid_baud_rate, required_unless_present = "keep_settings")]
        baud: Option<u32>,
        /// Path to the port to open
        #[arg(short, long)]
        port: String,
        /// Keeps the existing serial port configuration
        #[arg(short, long)]
        keep_settings: bool
    },
}

#[tokio::main]
async fn main() -> io::Result<()> {
    let cli = Cli::parse();
    match cli.command {
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
        Commands::PortSesh { baud, port, keep_settings}
        => {
            interactive_session(baud, &port, keep_settings).await?;
        }
    }
    Ok(())
}

async fn interactive_session(baud: Option<u32>, port: &str, keep_settings: bool) -> io::Result<()> {
    use tokio::{sync::Mutex, time::{timeout, Duration}};
    use std::sync::Arc;
    let mut stdout = io::stdout();
    terminal::enable_raw_mode()?;
    execute!(stdout, terminal::Clear(ClearType::All), cursor::SetCursorStyle::BlinkingBar, cursor::MoveTo(0,0)).ok();
    let (tx, mut rx) = tokio::sync::mpsc::channel::<Vec<u8>>(100);
    let con = Arc::new(Mutex::new(open_port(baud, port, keep_settings)?));

    // Reads the incoming serial data from the connection and sends it
    // to the `write_handle` via `mpsc::channel` to be written to stdout
    let read_con = Arc::clone(&con);
    let read_handle = tokio::spawn(async move {
        let mut buffer = vec![0u8; 1024];
        loop {
            let read_result = {
                let connection = read_con.lock().await;
                timeout(Duration::from_millis(100), connection.read(&mut buffer)).await 
            };
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
    });

    // Writes incoming serial data from the connection to stdout via `mpsc::channel`
    let print_stdout_handle = tokio::spawn(async move {
        // let mut stdout = tokio::io::stdout();
        while let Some(data) = rx.recv().await {
            let text = String::from_utf8_lossy(&data).to_string();
            tokio::task::spawn_blocking(move || {
                let mut stdout = io::stdout();
                execute!(stdout, Print(&text)).ok();
                stdout.flush().ok()
            }).await.ok();
            // if let Err(e) = stdout.write_all(text.as_bytes()).await {
            //     eprintln!("Write error: {e}");
            //     break;
            // };
            // stdout.flush().await.ok();
        }
    });

    // Reads data from stdin and sends it to the serial connection
    let write_con = Arc::clone(&con);
    let write_handle = tokio::spawn(async move {
        let (stdin_tx, mut stdin_rx) = tokio::sync::mpsc::channel::<String>(10);

        // Spawn blocking thread for stdin - use std::thread::spawn, not spawn_blocking
        std::thread::spawn(move || {
            // use std::io::{self, BufRead};
            // let stdin = io::stdin();
            // let handle = stdin.lock();
            // let mut reader = io::BufReader::new(handle);
            // let mut input = String::new();
            let mut current_line = String::new();
            let mut stdout = io::stdout();
            loop {
                if let Ok(true) = event::poll(Duration::from_millis(10)) {
                    match event::read().expect("should not fail because of `poll`") {
                        Event::Key(KeyEvent { code, modifiers, .. }) => {
                            match (code, modifiers) {
                                (KeyCode::Char('l'), KeyModifiers::CONTROL) => {
                                    execute!(stdout, terminal::Clear(ClearType::All), cursor::MoveTo(0,0)).ok();
                                }
                                (KeyCode::Char('q'), KeyModifiers::CONTROL) | (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                                    // TODO: HANDLE GRACEFULL PROCESS EXIT maybe just need to
                                    // return an error from this function?
                                    break;
                                }
                                (KeyCode::Enter, _) => {
                                    // if !current_line.is_empty() {
                                    //     execute!(stdout, Print(&current_line), Print("\r\n")).ok();
                                    // }
                                    execute!(stdout, crossterm::clipboard::CopyToClipboard::to_clipboard_from(&current_line)).ok();
                                    current_line.push('\r');
                                    if stdin_tx.blocking_send(current_line.clone()).is_err() {
                                        break;
                                    }
                                    current_line.clear();
                                }
                                (KeyCode::Backspace, _) => {
                                    if !current_line.is_empty() {
                                        current_line.pop();
                                        execute!(stdout, cursor::MoveLeft(1), Print(""), cursor::MoveLeft(1)).ok();
                                    }
                                }
                                (KeyCode::Char(c), _) => {
                                    current_line.push(c);
                                    stdin_tx.blocking_send(c.to_string()).ok();
                                    // execute!(stdout, Print(c)).ok();
                                }
                                _ => {}
                            }
                            stdout.flush().ok();
                        }
                        // Event::Mouse(event) => {
                        //     match event {
                        //         MouseEvent { kind: MouseEventKind::Down(MouseButton::Left), column, row, .. } => {
                        //         }
                        //         _ => {}
                        //     }
                        // }
                        _ => {}
                    }
                }
                // input.clear();
                // match reader.read_line(&mut input) {
                //     Ok(0) => break, // EOF
                //     Ok(_) => {
                //         eprintln!("DEBUG: Read from stdin: {input:?}"); // Debug line
                //         if stdin_tx.blocking_send(input.clone()).is_err() {
                //             break; // Receiver dropped
                //         }
                //     }
                //     Err(e) => {
                //         eprintln!("Input error: {e}");
                //         break;
                //     }
                // }
            }
        });

        // Async task receives from channel and writes to serial
        while let Some(data) = stdin_rx.recv().await {
            let connection = write_con.lock().await;
            connection.write_all(data.as_bytes()).await.ok();
            // let text = String::from_utf8_lossy(&data).to_string();
            // display_tx.send(text).ok();
        }
        // while let Some(input) = stdin_rx.recv().await {
        //     let connection = write_con.lock().await;
        //     let input_bytes = input.replace('\n', "\r").into_bytes();
        //     if let Err(e) = connection.write(&input_bytes).await {
        //         eprintln!("Serial write error: {e}");
        //         break;
        //     }
        // }
    });
    let result = tokio::try_join!(read_handle, print_stdout_handle, write_handle);
    // tokio::try_join!(read_handle, write_handle)?;
    terminal::disable_raw_mode()?;
    execute!(stdout, cursor::Show)?;
    result?;
    Ok(())
}

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
