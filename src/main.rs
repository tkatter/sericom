use std::{
    fs::File, io::{self, Write}
};
use clap::{CommandFactory, Parser, Subcommand};
use crossterm::{cursor, event::{self, Event, KeyCode, KeyEvent, KeyModifiers}, execute, style::Print, terminal::{self, ClearType} };
use serial2_tokio::SerialPort;

const UTF_TAB: &str = "\u{0009}";
const UTF_BKSP: &str = "\u{0008}";
const UTF_DEL: &str = "\u{007F}";
const UTF_ESC: &str = "\u{001B}";
const UTF_CTRL_C: &str = "\u{001B}\u{0043}";
const UTF_UP_KEY: &str = "\u{001B}\u{005B}\u{0041}";
const UTF_DOWN_KEY: &str = "\u{001B}\u{005B}\u{0042}";
const UTF_LEFT_KEY: &str = "\u{001B}\u{005B}\u{0044}";
const UTF_RIGHT_KEY: &str = "\u{001B}\u{005B}\u{0043}";

#[derive(Parser)]
#[command(name = "netcon", version, about, long_about = None)]
#[command(next_line_help = true)]
#[command(propagate_version = true)]
struct Cli {
    /// The path to a serial port.
    ///
    /// For Linux/MacOS something like `/dev/tty1`, Windows `COM1`.
    /// To see available ports, use `netcon list-ports`.
    port: Option<String>,
    /// Baud rate for the serial connection.
    ///
    /// To see a list of valid baud rates, use `netcon list-bauds`.
    #[arg(short, long, value_parser = valid_baud_rate, default_value_t = 9600)]
    baud: u32,
    /// Path to a file for the output.
    #[arg(short, long)]
    file: Option<String>,
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

#[derive(Debug)]
enum SerialMessage {
    Write(Vec<u8>),
    Shutdown,
}

#[derive(Debug, Clone)]
enum SerialEvent {
    Data(Vec<u8>),
    Error(String),
    ConnectionClosed,
}

struct SerialActor {
    connection: SerialPort,
    command_rx: tokio::sync::mpsc::Receiver<SerialMessage>,
    channels: Vec<tokio::sync::mpsc::Sender<SerialEvent>>
}

impl SerialActor {
    fn new (
        connection: SerialPort,
        command_rx: tokio::sync::mpsc::Receiver<SerialMessage>,
        channels: Vec<tokio::sync::mpsc::Sender<SerialEvent>>
    ) -> Self {
        Self {
            connection,
            command_rx,
            channels
        }
    }
    async fn broadcast_event(&self, event: SerialEvent) {
        for channel in &self.channels {
            let _ = channel.send(event.clone()).await;
        }
    }
    async fn run(mut self) {
        let mut buffer = vec![0u8; 1024];
        loop {
            tokio::select! {
                // Handle commands/input from stdin
                cmd = self.command_rx.recv() => {
                    match cmd {
                        Some(SerialMessage::Write(data)) => {
                            if let Err(e) = self.connection.write_all(&data).await {
                                self.broadcast_event(SerialEvent::Error(e.to_string())).await;
                            }
                        }
                        Some(SerialMessage::Shutdown) => {
                            self.broadcast_event(SerialEvent::ConnectionClosed).await;
                        }
                        None => break,
                    }
                }
                read_result = self.connection.read(&mut buffer) => {
                    match read_result {
                        Ok(0) => {
                            self.broadcast_event(SerialEvent::ConnectionClosed).await;
                            break;
                        }
                        Ok(n) => {
                            let data = buffer[..n].to_vec();
                            self.broadcast_event(SerialEvent::Data(data)).await;
                        }
                        Err(e) => {
                            self.broadcast_event(SerialEvent::Error(e.to_string())).await;
                            break;
                        }
                    }
                }
            }
        }
    }
}

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
        match temp_open_port(cli.baud, &port) {
            Err(e) => {
                if e.kind() == io::ErrorKind::NotFound {
                    let mut cmd = Cli::command();
                    cmd.error(
                        clap::error::ErrorKind::InvalidValue,
                        "The specified PORT is invalid. Use `netcon list-ports` to see a list of valid ports."
                    ).exit();
                }
            }
            Ok(con) => {
                interactive_session(con, cli.file).await?;
            }
        }
    } else if let Some(cmd) = cli.command {
        match cmd {
            Commands::ListBauds => {
                let mut stdout = io::stdout();
                write!(stdout, "Valid baud rates:\r\n")?;
                for baud in serial2_tokio::COMMON_BAUD_RATES {
                    write!(stdout, "{baud}\r\n")?;
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

async fn run_stdout_output(mut con_rx: tokio::sync::mpsc::Receiver<SerialEvent>) {
    let mut stdout = io::stdout();

    while let Some(event) = con_rx.recv().await {
        match event {
            // NOTE: May have errors here without using `tokio::task::spawn_blocking`
            SerialEvent::Data(data) => {
                execute!(
                    stdout,
                    crossterm::style::SetForegroundColor(crossterm::style::Color::Green),
                    Print(String::from_utf8_lossy(&data)),
                    ).ok();
                stdout.flush().ok();
            }
            SerialEvent::Error(e) => {
                eprintln!("SERIAL ERROR: {e}");
            }
            SerialEvent::ConnectionClosed => {
                eprintln!("\r\nConnection closed");
                break;
            }
        }
    }
}

async fn run_stdin_input(command_tx: tokio::sync::mpsc::Sender<SerialMessage>) {
    let (stdin_tx, mut stdin_rx) = tokio::sync::mpsc::channel::<String>(10);
    let shutdown_tx_clone = command_tx.clone();

    std::thread::spawn(move || {
        stdin_input_loop(stdin_tx, shutdown_tx_clone)
    });

    while let Some(data) = stdin_rx.recv().await {
        if command_tx.send(SerialMessage::Write(data.into_bytes())).await.is_err() {
            break;
        }
    }
}

fn stdin_input_loop(stdin_tx: tokio::sync::mpsc::Sender<String>, shutdown_tx: tokio::sync::mpsc::Sender<SerialMessage>) {
    loop {
        match event::read() {
            Ok(Event::Key(KeyEvent { code, modifiers, kind, .. })) => {
                if kind != crossterm::event::KeyEventKind::Press {
                    continue;
                }

                let data = match (code, modifiers) {
                    (KeyCode::Char('l'), KeyModifiers::CONTROL) => {
                        execute!(io::stdout(), terminal::Clear(ClearType::All), cursor::MoveTo(0,0)).ok();
                        continue;
                    }
                    (KeyCode::Char('q'), KeyModifiers::CONTROL) => {
                        let _ = shutdown_tx.blocking_send(SerialMessage::Shutdown);
                        break;
                    }
                    (KeyCode::Char('c'), KeyModifiers::CONTROL) => UTF_CTRL_C.to_string(),
                    (KeyCode::Tab, _) => UTF_TAB.to_string(),
                    (KeyCode::Delete, _) => UTF_DEL.to_string(),
                    (KeyCode::Up, _) => UTF_UP_KEY.to_string(),
                    (KeyCode::Down, _) => UTF_DOWN_KEY.to_string(),
                    (KeyCode::Left, _) => UTF_LEFT_KEY.to_string(),
                    (KeyCode::Right, _) => UTF_RIGHT_KEY.to_string(),
                    (KeyCode::Enter, _) => '\r'.to_string(),
                    (KeyCode::Backspace, _) => UTF_BKSP.to_string(),
                    (KeyCode::Esc, _) => UTF_ESC.to_string(),
                    (KeyCode::Char(c), _) => c.to_string(),
                    _ => continue,
                };

                if stdin_tx.blocking_send(data).is_err() {
                    break;
                }
            }
            Ok(_) => {} // Ignore other events
            Err(_) => break,
        }
    }
}
async fn run_file_output(mut file_rx: tokio::sync::mpsc::Receiver<SerialEvent>, filename: String) {
    let mut file = match File::create(&filename) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("Failed to create file '{filename}': {e}");
            return;
        }
    };

    writeln!(file, "Session started at: {}", chrono::Utc::now()).ok();

    while let Some(event) = file_rx.recv().await {
        match event {
            SerialEvent::Data(data) => {
                file.write_all(&data).ok();
                file.flush().ok();
            }
            SerialEvent::Error(e) => {
                writeln!(file, "\r\n[ERROR {}] {}", chrono::Utc::now(), e).ok();
            }
            SerialEvent::ConnectionClosed => {
                writeln!(file, "\r\n[CLOSED {}] Connection closed.", chrono::Utc::now()).ok();
                break;
            }
        }
    }
}

async fn interactive_session(connection: SerialPort, file: Option<String>) -> io::Result<()> {
    // Setup terminal
    let mut stdout = io::stdout();
    execute!(stdout, terminal::EnterAlternateScreen)?;
    terminal::enable_raw_mode()?;
    execute!(stdout, terminal::Clear(ClearType::All), cursor::MoveTo(0,0)).ok();

    // Create channels
    let (command_tx, command_rx) = tokio::sync::mpsc::channel::<SerialMessage>(100);
    let (stdout_tx, stdout_rx) = tokio::sync::mpsc::channel::<SerialEvent>(100);

    let mut channels = vec![stdout_tx];
    let mut tasks = Vec::new();

    if let Some(filename) = file {
        let (file_tx, file_rx) = tokio::sync::mpsc::channel::<SerialEvent>(100);
        channels.push(file_tx);
        tasks.push(tokio::spawn(run_file_output(file_rx, filename)));
    }

    // Create and spawn SerialActor
    let actor = SerialActor::new(connection, command_rx, channels);
    let actor_handle = tokio::spawn(actor.run());

    // Spawn output and input tasks
    let stdout_task = tokio::spawn(run_stdout_output(stdout_rx));
    let stdin_task = tokio::spawn(run_stdin_input(command_tx));

    tasks.push(actor_handle);
    tasks.push(stdout_task);
    tasks.push(stdin_task);

    // Wait for tasks to complete
    for task in tasks {
        let _ = task.await;
    }

    ensure_terminal_cleanup(stdout);
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

fn get_settings(baud: u32, port: &str) -> Result<(), io::Error> {
    // https://www.contec.com/support/basic-knowledge/daq-control/serial-communicatin/
    let mut stdout = io::stdout();
    let con = temp_open_port(baud, port)?;
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

    write!(stdout, "Baud rate: {b}\r\n")?;
    write!(stdout, "Char size: {c}\r\n")?;
    write!(stdout, "Stop bits: {s}\r\n")?;
    write!(stdout, "Parity mechanism: {p}\r\n")?;
    write!(stdout, "Flow control: {f}\r\n")?;
    write!(stdout, "Clear To Send line: {cts}\r\n")?;
    write!(stdout, "Data Set Ready line: {dsr}\r\n")?;
    write!(stdout, "Ring Indicator line: {ri}\r\n")?;
    write!(stdout, "Carrier Detect line: {cd}\r\n")?;

    Ok(())
}

fn list_serial_ports() -> Result<(), io::Error> {
    let mut stdout = io::stdout();
    let ports = SerialPort::available_ports()?;
    for path in ports {
        if let Some(path) = path.to_str() {
            let line = [path, "\r\n"].concat();
            stdout.write(line.as_bytes())?
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
