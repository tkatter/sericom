use std::{
    fs::File, io::{self, BufWriter, Write}, sync::Arc
};
use clap::{CommandFactory, Parser, Subcommand};
use crossterm::{cursor, event::{self, Event, KeyCode, KeyEvent, KeyModifiers, MouseEvent}, execute, terminal::{self, ClearType} };
use serial2_tokio::SerialPort;
use netcon::screen_buffer::{ScreenBuffer, UICommand};

const UTF_TAB: &str = "\u{0009}";
const UTF_BKSP: &str = "\u{0008}";
const UTF_DEL: &str = "\u{007F}";
const UTF_ESC: &str = "\u{001B}";
const UTF_CTRL_C: &str = "\u{03}";
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

#[derive(Debug)]
enum SerialMessage {
    Write(Vec<u8>),
    Shutdown,
}

#[derive(Clone, Debug)]
enum SerialEvent {
    Data(Arc<[u8]>),
    Error(String),
    ConnectionClosed,
}

struct SerialActor {
    connection: SerialPort,
    command_rx: tokio::sync::mpsc::Receiver<SerialMessage>,
    broadcast_channel: tokio::sync::broadcast::Sender<SerialEvent>,
}

impl SerialActor {
    fn new (
        connection: SerialPort,
        command_rx: tokio::sync::mpsc::Receiver<SerialMessage>,
        broadcast_channel: tokio::sync::broadcast::Sender<SerialEvent>
    ) -> Self {
        Self {
            connection,
            command_rx,
            broadcast_channel,
        }
    }

    async fn run(mut self) {
        let mut buffer = vec![0u8; 4096];
        loop {
            tokio::select! {
                // Handle commands/input from tasks
                cmd = self.command_rx.recv() => {
                    match cmd {
                        Some(SerialMessage::Write(data)) => {
                            if let Err(e) = self.connection.write_all(&data).await {
                                self.broadcast_channel.send(SerialEvent::Error(e.to_string())).ok();
                            }
                        }
                        Some(SerialMessage::Shutdown) => {
                            self.broadcast_channel.send(SerialEvent::ConnectionClosed).ok();
                        }
                        None => break,
                    }
                }
                // Handle reading data from serial connection
                read_result = self.connection.read(&mut buffer) => {
                    match read_result {
                        Ok(0) => {
                            self.broadcast_channel.send(SerialEvent::ConnectionClosed).ok();
                            break;
                        }
                        Ok(n) => {
                            let data: Arc<[u8]> = buffer[..n].into();
                            self.broadcast_channel.send(SerialEvent::Data(data)).ok();
                        }
                        Err(e) => {
                            self.broadcast_channel.send(SerialEvent::Error(e.to_string())).ok();
                            break;
                        }
                    }
                }
            }
        }
    }
}

async fn run_debug_output(mut rx: tokio::sync::broadcast::Receiver<SerialEvent>) {
    let (write_tx, write_rx) = std::sync::mpsc::channel::<Vec<u8>>();
    let write_handle = tokio::task::spawn_blocking(move || {
        let file = match File::create("debug.txt") {
            Ok(f) => f,
            Err(e) => {
                eprintln!("Failed to create file: {e}");
                return;
            }
        };
        let mut writer = BufWriter::with_capacity(48 * 1024, file);
        let mut last_flush = std::time::Instant::now();

        writeln!(writer, "Session started at: {}", chrono::Utc::now()).ok();
        while let Ok(data) = write_rx.recv() {
            writeln!(writer,
                "[{}] RX {} bytes: {:02X?}{} UTF8: {}",
                chrono::Utc::now().format("%H:%M:%S%.3f"),
                data.len(),
                &data[..std::cmp::min(8, data.len())],
                if data.len() > 8 { "..." } else { "" },
                String::from_utf8_lossy(&data)
            ).ok();

            let now = std::time::Instant::now();
            if now.duration_since(last_flush) > std::time::Duration::from_millis(100)
                || writer.buffer().len() > 32 * 1024 {
                    let _ = writer.flush();
                    last_flush = now;
            }
        }
        let _ = writer.flush();
    });


    let data_streamer = tokio::spawn(async move {
        let mut write_buf = Vec::with_capacity(4096);
        let mut batch_timer = tokio::time::interval(tokio::time::Duration::from_millis(200));

        loop {
            tokio::select! {
                event = rx.recv() => {
                    match event {
                        Ok(SerialEvent::Data(data)) => {
                            write_buf.extend_from_slice(&data);
                            if write_buf.len() >= 4096 && write_tx.send(std::mem::take(&mut write_buf)).is_err() {
                                    break;
                            }
                        }
                        // SerialEvent::Error(e) => {
                        //     println!("[{}] ERROR: {}", chrono::Utc::now().format("%H:%M:%S%.3f"), e);
                        //     writer.flush().ok();
                        // }
                        // SerialEvent::ConnectionClosed => {
                        //     println!("[{}] Connection closed", chrono::Utc::now().format("%H:%M:%S%.3f"));
                        //     writer.flush().ok();
                        //     break;
                        // }
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped)) => {
                            eprintln!("File writer lagged, skipped {skipped} messages");
                            continue; // Don't break on lag
                        }
                        _ => break,
                    }
                }
                _ = batch_timer.tick() => {
                    if !write_buf.is_empty() && write_tx.send(std::mem::take(&mut write_buf)).is_err() {
                            break;
                    }
                }
            }
        }
        if !write_buf.is_empty() { let _ = write_tx.send(std::mem::take(&mut write_buf));
        }
        drop(write_tx);
    });

    let _ = data_streamer.await;
    let _ = write_handle.await;
}

#[tokio::main]
async fn main() -> io::Result<()> {
    // console_subscriber::init();
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
        match open_connection(cli.baud, &port) {
            Err(e) => {
                match e.kind() {
                    io::ErrorKind::NotFound => {
                        let mut cmd = Cli::command();
                        cmd.error(
                            clap::error::ErrorKind::InvalidValue,
                            "The specified PORT is invalid. Use `netcon list-ports` to see a list of valid ports."
                        ).exit();
                    }
                    e => {
                        let mut cmd = Cli::command();
                        let message = format!("{e}");
                        cmd.error(
                            clap::error::ErrorKind::InvalidValue,
                            message
                        ).exit();
                    }
                }
            }
            Ok(con) => {
                interactive_session(con, cli.file, cli.debug, &port).await?;
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

async fn run_stdout_output(mut con_rx: tokio::sync::broadcast::Receiver<SerialEvent>, mut ui_rx: tokio::sync::mpsc::Receiver<UICommand>) {
    let (width, height) = terminal::size().unwrap_or((80, 24));
    let mut screen_buffer = ScreenBuffer::new(width, height, 10000);
    let mut data_buffer = Vec::with_capacity(2048);
    let mut render_timer: Option<tokio::time::Interval> = None;

    loop {
        tokio::select!{
            serial_event = con_rx.recv() => {
                match serial_event {
                    Ok(SerialEvent::Data(data)) => {
                        data_buffer.extend_from_slice(&data);

                        if data_buffer.len() > 1024 || data.contains(&b'\n') {
                            screen_buffer.add_data(&data_buffer);
                            data_buffer.clear();

                            if screen_buffer.should_render_now() {
                                screen_buffer.render().ok();
                                render_timer = None;
                            } else if render_timer.is_none() {
                                render_timer = Some(tokio::time::interval(tokio::time::Duration::from_millis(16)));
                            }
                        } else {
                            screen_buffer.add_data(&data_buffer);
                            data_buffer.clear();

                            if screen_buffer.should_render_now() {
                                screen_buffer.render().ok();
                            } else if render_timer.is_none() {
                                render_timer = Some(tokio::time::interval(tokio::time::Duration::from_millis(16)));
                            }
                        }
                    }
                    Ok(SerialEvent::Error(e)) => {
                        let error_msg = format!("[ERROR] {e}\r\n");
                        screen_buffer.add_data(error_msg.as_bytes());
                        screen_buffer.render().ok();
                        render_timer = None;
                    }
                    Ok(SerialEvent::ConnectionClosed) => break,
                    Err(_) => break,
                }
            }
            ui_command = ui_rx.recv() => {
                match ui_command {
                    Some(UICommand::ScrollUp(lines)) => {
                        screen_buffer.scroll_up(lines);
                        screen_buffer.render().ok();
                        render_timer = None;
                    }
                    Some(UICommand::ScrollDown(lines)) => {
                        screen_buffer.scroll_down(lines);
                        screen_buffer.render().ok();
                        render_timer = None;
                    }
                    Some(UICommand::StartSelection(x, y)) => {
                        screen_buffer.start_selection(x, y);
                        screen_buffer.render().ok();
                        render_timer = None;
                    }
                    Some(UICommand::UpdateSelection(x, y)) => {
                        screen_buffer.update_selection(x, y);
                        screen_buffer.render().ok();
                        render_timer = None;
                    }
                    Some(UICommand::CopySelection) => {
                        screen_buffer.copy_to_clipboard().ok();
                        screen_buffer.render().ok();
                        render_timer = None;
                    }
                    Some(UICommand::ClearBuffer) => {
                        screen_buffer.clear_buffer();
                        screen_buffer.render().ok();
                        render_timer = None;
                    }
                    None => break,
                }
            }
            _ = async {
                if let Some(ref mut timer) = render_timer {
                    timer.tick().await;
                } else {
                    std::future::pending::<()>().await
                }
            } => {
                if screen_buffer.should_render_now() {
                    screen_buffer.render().ok();
                    render_timer = None;
                }
            }
        }
    }
}

async fn run_stdin_input(command_tx: tokio::sync::mpsc::Sender<SerialMessage>, ui_tx: tokio::sync::mpsc::Sender<UICommand>) {
    let (stdin_tx, mut stdin_rx) = tokio::sync::mpsc::channel::<String>(10);
    let command_tx_clone = command_tx.clone();

    tokio::task::spawn_blocking(move || {
        stdin_input_loop(stdin_tx, command_tx_clone, ui_tx)
    });

    while let Some(data) = stdin_rx.recv().await {
        if command_tx.send(SerialMessage::Write(data.into_bytes())).await.is_err() {
            break;
        }
    }
}

fn stdin_input_loop(stdin_tx: tokio::sync::mpsc::Sender<String>, command_tx: tokio::sync::mpsc::Sender<SerialMessage>, ui_tx: tokio::sync::mpsc::Sender<UICommand>) {
    loop {
        match event::read() {
            // Match function keys
            Ok(Event::Key(KeyEvent { code: KeyCode::F(f_code), modifiers: _modifiers, kind, .. })) => {
                if kind != crossterm::event::KeyEventKind::Press {
                    continue;
                }
                match f_code {
                    1 => {
                        let term_len = "terminal length 0\r".to_string();
                        if stdin_tx.blocking_send(term_len).is_err() {
                            break;
                        }
                        let test_report = "show inventory\rshow version\rshow license summary\rshow license usage\rshow environment all\rshow power inline\rshow interface status\rshow diagnostic post\rshow diagnostic result switch all\r".to_string();
                        if stdin_tx.blocking_send(test_report).is_err() {
                            break;
                        }
                    },
                    2 => {
                        let term_len = "terminal length 0\r".to_string();
                        if stdin_tx.blocking_send(term_len).is_err() {
                            break;
                        }
                        let test_report = "show inventory\rshow version\rshow license\rshow license usage\rshow environment\rshow startup-config\rshow interface status\rshow boot\rshow diagnostic result all\r".to_string();
                        if stdin_tx.blocking_send(test_report).is_err() {
                            break;
                        }
                    },
                    _ => continue,
                };
            }
            // Match Control + Code
            Ok(Event::Key(KeyEvent { code, modifiers: KeyModifiers::CONTROL, kind, .. })) => {
                if kind != crossterm::event::KeyEventKind::Press {
                    continue;
                }
                match code {
                    KeyCode::Char('c') => {
                        let _ = stdin_tx.blocking_send(UTF_CTRL_C.to_string());
                        continue;
                    }
                    KeyCode::Char('l') => {
                        let _ = ui_tx.blocking_send(UICommand::ClearBuffer);
                        continue;
                    }
                    KeyCode::Char('q') => {
                        let _ = command_tx.blocking_send(SerialMessage::Shutdown);
                        break;
                    }
                    _ => continue,
                };
            }
            // Match every other key
            Ok(Event::Key(KeyEvent { code, modifiers: _, kind, .. })) => {
                if kind != crossterm::event::KeyEventKind::Press {
                    continue;
                }
                let data = match code {
                    KeyCode::Tab => UTF_TAB.to_string(),
                    KeyCode::Delete => UTF_DEL.to_string(),
                    KeyCode::Up => UTF_UP_KEY.to_string(),
                    KeyCode::Down => UTF_DOWN_KEY.to_string(),
                    KeyCode::Left => UTF_LEFT_KEY.to_string(),
                    KeyCode::Right => UTF_RIGHT_KEY.to_string(),
                    KeyCode::Enter => '\r'.to_string(),
                    KeyCode::Backspace => UTF_BKSP.to_string(),
                    KeyCode::Esc => UTF_ESC.to_string(),
                    KeyCode::Char(c) => c.to_string(),
                    _ => continue,
                };

                if stdin_tx.blocking_send(data).is_err() {
                    break;
                }
            }
            Ok(Event::Mouse(MouseEvent { kind, column, row, .. })) => {
                let ui_command = match kind {
                    event::MouseEventKind::ScrollUp => UICommand::ScrollUp(1),
                    event::MouseEventKind::ScrollDown => UICommand::ScrollDown(1),
                    event::MouseEventKind::Down(_) => UICommand::StartSelection(column, row),
                    event::MouseEventKind::Drag(_) => UICommand::UpdateSelection(column, row),
                    event::MouseEventKind::Up(_) => UICommand::CopySelection,
                    _ => continue,
                };
                if ui_tx.blocking_send(ui_command).is_err() {
                    break;
                }
            }
            Ok(Event::Paste(text)) => {
                if stdin_tx.blocking_send(text).is_err() {
                    break;
                }
            }
            Ok(_) => {} // Ignore other events
            Err(_) => break,
        }
    }
}

async fn run_file_output(mut file_rx: tokio::sync::broadcast::Receiver<SerialEvent>, filename: String) {
    let (write_tx, write_rx) = std::sync::mpsc::channel::<Vec<u8>>();
    let filename_clone = filename.clone();

    let write_handle = tokio::task::spawn_blocking(move || {
        let file = match File::create(&filename_clone) {
            Ok(f) => f,
            Err(e) => {
                eprintln!("Failed to create file '{filename_clone}': {e}");
                return;
            }
        };
        let mut writer = BufWriter::with_capacity(8 * 1024, file);
        let mut last_flush = std::time::Instant::now();

        writeln!(writer, "SERIAL: ").ok();
        writeln!(writer, "Session started at: {}", chrono::Utc::now()).ok();
        while let Ok(data) = write_rx.recv() {
            writer.write_all(&data).ok();

            let now = std::time::Instant::now();
            if now.duration_since(last_flush) > std::time::Duration::from_millis(200)
                || writer.buffer().len() > 4 * 1024 {
                    let _ = writer.flush();
                    last_flush = now;
            }
        }
        let _ = writer.flush();
    });


    let data_streamer = tokio::spawn(async move {
        let mut write_buf = Vec::with_capacity(4096);
        let mut batch_timer = tokio::time::interval(tokio::time::Duration::from_millis(200));

        loop {
            tokio::select! {
                event = file_rx.recv() => {
                    match event {
                        Ok(SerialEvent::Data(data)) => {
                            write_buf.extend_from_slice(&data);

                            if write_buf.len() >= 4096 && write_tx.send(std::mem::take(&mut write_buf)).is_err() {
                                    break;
                            }
                        }
                        Ok(SerialEvent::Error(e)) => {
                            if !write_buf.is_empty() {
                                if write_tx.send(std::mem::take(&mut write_buf)).is_err() {
                                    break;
                                }
                                write_buf.clear();
                            }
                            let error_msg = format!("\r\n[ERROR {}] {e}\r\n", chrono::Utc::now());
                            let _ = write_tx.send(error_msg.into_bytes());
                        }
                        Ok(SerialEvent::ConnectionClosed) => {
                            if !write_buf.is_empty() {
                                if write_tx.send(std::mem::take(&mut write_buf)).is_err() {
                                    break;
                                }
                                write_buf.clear();
                            }
                            let close_msg = format!("\r\n[CLOSED {}] Connection closed.\r\n", chrono::Utc::now());
                            let _ = write_tx.send(close_msg.into_bytes());
                            break;
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped)) => {
                            eprintln!("File writer lagged, skipped {skipped} messages");
                            continue; // Don't break on lag
                        }
                        _ => break,
                    }
                }
                _ = batch_timer.tick() => {
                    if !write_buf.is_empty() && write_tx.send(std::mem::take(&mut write_buf)).is_err() {
                            break;
                    }
                }
            }
        }
        if !write_buf.is_empty() { let _ = write_tx.send(std::mem::take(&mut write_buf));
        }
        drop(write_tx);
    });

    let _ = data_streamer.await;
    let _ = write_handle.await;
}

async fn interactive_session(connection: SerialPort, file: Option<String>, debug: bool, port_name: &str) -> io::Result<()> {
    // Setup terminal
    let mut stdout = io::stdout();
    terminal::enable_raw_mode()?;
    execute!(stdout,
        terminal::EnterAlternateScreen,
        terminal::SetTitle(port_name),
        terminal::Clear(ClearType::All),
        event::EnableBracketedPaste,
        event::EnableMouseCapture,
        cursor::MoveTo(0,0)
    )?;

    // Create channels
    let (command_tx, command_rx) = tokio::sync::mpsc::channel::<SerialMessage>(100);
    let (ui_tx, ui_rx) = tokio::sync::mpsc::channel::<UICommand>(100);
    let (broadcast_event_tx, _) = tokio::sync::broadcast::channel::<SerialEvent>(128);
    let stdout_rx = broadcast_event_tx.subscribe();

    // Create tasks
    let mut tasks = tokio::task::JoinSet::new();

    if let Some(filename) = file {
        let file_rx = broadcast_event_tx.subscribe();
        tasks.spawn(run_file_output(file_rx, filename));
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

fn open_connection(baud: u32, port: &str) -> io::Result<SerialPort> {
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
    let con = open_connection(baud, port)?;
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
    let _ = execute!(stdout, event::DisableMouseCapture, event::DisableBracketedPaste, LeaveAlternateScreen, Show);
    let _ = disable_raw_mode();
    let _ = stdout.flush();
}
