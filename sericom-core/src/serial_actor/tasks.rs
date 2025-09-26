use crate::{
    screen_buffer::*,
    serial_actor::{SerialEvent, SerialMessage, parser::ByteParser},
    ui::{Rect, Terminal},
};
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind},
    terminal,
};
use std::{
    fs::File,
    io::{BufWriter, Write},
    path::PathBuf,
};
use tracing::{error, info, instrument};

const UTF_TAB: &str = "\u{0009}";
const UTF_BKSP: &str = "\u{0008}";
const UTF_DEL: &str = "\u{007F}";
const UTF_ESC: &str = "\u{001B}";
const UTF_CTRL_C: &str = "\u{03}";
const UTF_UP_KEY: &str = "\u{001B}\u{005B}\u{0041}";
const UTF_DOWN_KEY: &str = "\u{001B}\u{005B}\u{0042}";
const UTF_LEFT_KEY: &str = "\u{001B}\u{005B}\u{0044}";
const UTF_RIGHT_KEY: &str = "\u{001B}\u{005B}\u{0043}";

/// Responsible for receiving incoming data from the [`SerialActor`] and
/// rendering terminal output via the [`ScreenBuffer`].
#[instrument(skip_all, name = "Stdout")]
pub async fn run_stdout_output(
    mut con_rx: tokio::sync::broadcast::Receiver<SerialEvent>,
    mut ui_rx: tokio::sync::mpsc::Receiver<UICommand>,
) {
    let (width, height) = terminal::size().unwrap_or((80, 24));
    let mut screen_buffer = ScreenBuffer::new(Rect {
        width,
        height,
        origin: crate::ui::Position::ORIGIN,
    });
    let mut data_buffer = Vec::with_capacity(2048);
    let mut render_timer: Option<tokio::time::Interval> = None;

    loop {
        tokio::select! {
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
                        } else if screen_buffer.should_render_now() {
                            screen_buffer.add_data(&data_buffer);
                            data_buffer.clear();

                            screen_buffer.render().ok();
                        } else if render_timer.is_none() {
                            render_timer = Some(tokio::time::interval(tokio::time::Duration::from_millis(16)));
                        }
                    }
                    Ok(SerialEvent::Error(e)) => {
                        let error_msg = format!("[ERROR] {e}\r\n");
                        error!("Added data: {:?}", data_buffer);
                        screen_buffer.add_data(error_msg.as_bytes());
                        screen_buffer.render().ok();
                        render_timer = None;
                    }
                    Ok(SerialEvent::ConnectionClosed) | Err(_) => break,
                }
            }
            ui_command = ui_rx.recv() => {
                // debug!("Sending UICommand: {:?}", ui_command);
                match ui_command {
                    Some(UICommand::ScrollUp(lines)) => {
                        screen_buffer.scroll_up(lines);
                    }
                    Some(UICommand::ScrollDown(lines)) => {
                        screen_buffer.scroll_down(lines);
                    }
                    Some(UICommand::ScrollTop) => {
                        screen_buffer.scroll_to_top();
                    }
                    Some(UICommand::ScrollBottom) => {
                        screen_buffer.scroll_to_bottom();
                    }
                    Some(UICommand::StartSelection(pos)) => {
                        screen_buffer.start_selection(pos);
                    }
                    Some(UICommand::UpdateSelection(pos)) => {
                        screen_buffer.update_selection(pos);
                    }
                    Some(UICommand::CopySelection) => {
                        screen_buffer.copy_to_clipboard().ok();
                    }
                    Some(UICommand::ClearBuffer) => {
                        screen_buffer.clear_buffer();
                    }
                    None => break,
                }
                screen_buffer.render().ok();
                render_timer = None;
            }
            () = async {
                if let Some(ref mut timer) = render_timer {
                    timer.tick().await;
                } else {
                    std::future::pending::<()>().await;
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

/// Responsible for spawning a blocking task with [`tokio::task::spawn_blocking()`]
/// and processing user input from stdin.
///
/// Sends data via [`SerialMessage`] to the serial connection and
/// [`UICommand`]s to the [`ScreenBuffer`] for processing user actions like
/// scrolling, copying, clearing the screen, etc.
pub async fn run_stdin_input(
    command_tx: tokio::sync::mpsc::Sender<SerialMessage>,
    ui_tx: tokio::sync::mpsc::Sender<UICommand>,
) {
    let (stdin_tx, mut stdin_rx) = tokio::sync::mpsc::channel::<String>(10);
    let command_tx_clone = command_tx.clone();

    tokio::task::spawn_blocking(move || stdin_input_loop(stdin_tx, command_tx_clone, ui_tx));

    while let Some(data) = stdin_rx.recv().await {
        if command_tx
            .send(SerialMessage::Write(data.into_bytes()))
            .await
            .is_err()
        {
            break;
        }
    }
}

#[allow(clippy::too_many_lines)]
#[instrument(skip_all, name = "Stdin Input")]
fn stdin_input_loop(
    stdin_tx: tokio::sync::mpsc::Sender<String>,
    command_tx: tokio::sync::mpsc::Sender<SerialMessage>,
    ui_tx: tokio::sync::mpsc::Sender<UICommand>,
) {
    while let Ok(event) = event::read() {
        tracing::debug!("Read: '{:?}'", event);
        match event {
            // Match function keys
            Event::Key(KeyEvent {
                code: KeyCode::F(f_code),
                modifiers: _modifiers,
                kind,
                ..
            }) => {
                if kind != crossterm::event::KeyEventKind::Press {
                    continue;
                }
                match f_code {
                    1 => {
                        let _ = ui_tx.blocking_send(UICommand::ScrollTop);
                    }
                    2 => {
                        let _ = ui_tx.blocking_send(UICommand::ScrollBottom);
                    }
                    _ => {}
                };
            }
            // Match Alt + Code
            Event::Key(KeyEvent {
                code,
                modifiers: KeyModifiers::ALT,
                kind,
                ..
            }) => {
                if kind != crossterm::event::KeyEventKind::Press {
                    continue;
                }
                if let KeyCode::Char('b') = code {
                    let _ = command_tx.blocking_send(SerialMessage::SendBreak);
                };
            }
            // Match Control + Code
            Event::Key(KeyEvent {
                code,
                modifiers: KeyModifiers::CONTROL,
                kind,
                ..
            }) => {
                if kind != crossterm::event::KeyEventKind::Press {
                    continue;
                }
                match code {
                    KeyCode::Char('c') => {
                        let _ = stdin_tx.blocking_send(UTF_CTRL_C.to_string());
                    }
                    KeyCode::Char('l') => {
                        let _ = ui_tx.blocking_send(UICommand::ClearBuffer);
                    }
                    KeyCode::Char('q') => {
                        let _ = command_tx.blocking_send(SerialMessage::Shutdown);
                        break;
                    }
                    _ => {}
                };
            }
            // Match every other key
            Event::Key(KeyEvent {
                code,
                modifiers: _,
                kind,
                ..
            }) => {
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
            Event::Mouse(MouseEvent {
                kind, column, row, ..
            }) => {
                let ui_command = match kind {
                    MouseEventKind::ScrollUp => UICommand::ScrollUp(1),
                    MouseEventKind::ScrollDown => UICommand::ScrollDown(1),
                    MouseEventKind::Down(_) => UICommand::StartSelection((column, row).into()),
                    MouseEventKind::Drag(_) => UICommand::UpdateSelection((column, row).into()),
                    MouseEventKind::Up(_) => UICommand::CopySelection,
                    _ => continue,
                };
                if ui_tx.blocking_send(ui_command).is_err() {
                    break;
                }
            }
            Event::Paste(text) => {
                if stdin_tx.blocking_send(text).is_err() {
                    break;
                }
            }
            _ => {} // Ignore other events
        }
    }
}

/// Responsible for spawning a blocking task with [`tokio::task::spawn_blocking()`]
/// and forwarding the incoming data received from the [`SerialActor`] to the blocking
/// task to write to a file.
#[instrument(name = "File output", skip(file_rx))]
pub async fn run_file_output(
    mut file_rx: tokio::sync::broadcast::Receiver<SerialEvent>,
    file_path: PathBuf,
) {
    let (write_tx, write_rx) = std::sync::mpsc::channel::<Vec<u8>>();
    info!("Creating file: '{}'", file_path.display());
    let write_handle = tokio::task::spawn_blocking(move || {
        let file = match File::create(&file_path) {
            Ok(f) => f,
            Err(e) => {
                eprintln!("Failed to create file '{}': {e}", file_path.display());
                return;
            }
        };
        let mut writer = BufWriter::with_capacity(8 * 1024, file);
        let mut last_flush = std::time::Instant::now();

        writeln!(writer, "Session started at: {}", chrono::Utc::now()).ok();
        while let Ok(data) = write_rx.recv() {
            writer.write_all(&data).ok();
            let now = std::time::Instant::now();
            if now.duration_since(last_flush) > std::time::Duration::from_millis(200)
                || writer.buffer().len() > 4 * 1024
            {
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
        if !write_buf.is_empty() {
            let _ = write_tx.send(std::mem::take(&mut write_buf));
        }
        drop(write_tx);
    });

    let _ = data_streamer.await;
    let _ = write_handle.await;
}

pub async fn _run_stdout_output(
    mut con_rx: tokio::sync::broadcast::Receiver<SerialEvent>,
    mut ui_rx: tokio::sync::mpsc::Receiver<UICommand>,
) {
    let mut terminal = Terminal::default();
    let screen_buffer = ScreenBuffer::new(terminal.area());
    let mut data_buffer: Vec<u8> = Vec::with_capacity(2048);
    let mut render_timer: Option<tokio::time::Interval> = None;

    /////////
    let mut parser = ByteParser::new();

    terminal.draw(|frame| {
        frame.render_widget(screen_buffer, frame.area());
    });

    loop {
        tokio::select! {
            serial_event = con_rx.recv() => {
                match serial_event {
                    Ok(SerialEvent::Data(data)) => {
                        data_buffer.extend_from_slice(&data);
                        // screen_buffer.add_data(&data_buffer);
                        // data_buffer.clear();
                        if data_buffer.len() > 1024 || data.contains(&b'\n') {
                            let parsed = parser.feed(&data_buffer);
                            tracing::debug!("{:?}", parsed);
                            data_buffer.clear();

                            // if screen_buffer.should_render_now() {
                            //     screen_buffer.render().ok();
                            //     render_timer = None;
                            // } else if render_timer.is_none() {
                            //     render_timer = Some(tokio::time::interval(tokio::time::Duration::from_millis(16)));
                            // }
                        }
                        // } else if &screen_buffer.should_render_now() {
                        //     let parsed = parser.feed(&data_buffer);
                        //     tracing::debug!("{:?}", parsed);
                        //     data_buffer.clear();
                        //
                        //     screen_buffer.render().ok();
                        // } else if render_timer.is_none() {
                        //     render_timer = Some(tokio::time::interval(tokio::time::Duration::from_millis(16)));
                        // }
                    }
                    Ok(SerialEvent::Error(_)) => {
                        // let error_msg = format!("[ERROR] {e}\r\n");
                        // error!("Added data: {:?}", data_buffer);
                        // screen_buffer.add_data(error_msg.as_bytes());
                        // screen_buffer.render().ok();
                        // render_timer = None;
                    }
                    Ok(SerialEvent::ConnectionClosed) | Err(_) => break,
                }
            }
            ui_command = ui_rx.recv() => {
                // debug!("Sending UICommand: {:?}", ui_command);
                // match ui_command {
                //     Some(UICommand::ScrollUp(lines)) => {
                //         screen_buffer.scroll_up(lines);
                //     }
                //     Some(UICommand::ScrollDown(lines)) => {
                //         screen_buffer.scroll_down(lines);
                //     }
                //     Some(UICommand::ScrollTop) => {
                //         screen_buffer.scroll_to_top();
                //     }
                //     Some(UICommand::ScrollBottom) => {
                //         screen_buffer.scroll_to_bottom();
                //     }
                //     Some(UICommand::StartSelection(pos)) => {
                //         screen_buffer.start_selection(pos);
                //     }
                //     Some(UICommand::UpdateSelection(pos)) => {
                //         screen_buffer.update_selection(pos);
                //     }
                //     Some(UICommand::CopySelection) => {
                //         screen_buffer.copy_to_clipboard().ok();
                //     }
                //     Some(UICommand::ClearBuffer) => {
                //         screen_buffer.clear_buffer();
                //     }
                //     None => break,
                // }
                // screen_buffer.render().ok();
                // render_timer = None;
            }
            () = async {
                if let Some(ref mut timer) = render_timer {
                    timer.tick().await;
                } else {
                    std::future::pending::<()>().await;
                }
            } => {
                // if screen_buffer.should_render_now() {
                //     screen_buffer.render().ok();
                //     render_timer = None;
                // }
            }
        }
    }
}
