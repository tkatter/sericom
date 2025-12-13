use crate::{screen::UICommand, serial_actor::SerialMessage};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind};

use tracing::instrument;

const UTF_TAB: &str = "\u{0009}";
const UTF_BKSP: &str = "\u{0008}";
const UTF_DEL: &str = "\u{007F}";
const UTF_ESC: &str = "\u{001B}";
const UTF_CTRL_C: &str = "\u{03}";
const UTF_UP_KEY: &str = "\u{001B}\u{005B}\u{0041}";
const UTF_DOWN_KEY: &str = "\u{001B}\u{005B}\u{0042}";
const UTF_LEFT_KEY: &str = "\u{001B}\u{005B}\u{0044}";
const UTF_RIGHT_KEY: &str = "\u{001B}\u{005B}\u{0043}";

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
                }
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
                if KeyCode::Char('b') == code {
                    let _ = command_tx.blocking_send(SerialMessage::SendBreak);
                }
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
                }
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
