//! This module holds all of the code directly responsible for interacting
//! with the serial connection and tasks within the program.

use tracing::{Level, span};

pub mod tasks;

/// Represents messages/commands that are sent from worker tasks
/// to the [`SerialActor`] to process.
#[non_exhaustive]
#[derive(Debug)]
pub enum SerialMessage {
    /// Instructs the [`SerialActor`] to write bytes (`Vec<u8>`) to the serial connection.
    Write(Vec<u8>),
    /// Instructs the [`SerialActor`] to send a 'break' signal over the serial connection.
    SendBreak,
    /// Instructs the [`SerialActor`] to shutdown the serial connection.
    Shutdown,
}

/// Represents events from the [`SerialActor`] that will be
/// received and processed by worker tasks accordingly.
#[non_exhaustive]
#[derive(Clone, Debug)]
pub enum SerialEvent {
    /// Sends data received by the [`SerialActor`] to its tasks.
    Data(std::sync::Arc<[u8]>),
    /// Sends the error message received by the [`SerialActor`] to its tasks to handle.
    Error(String),
    /// Tells the [`SerialActor`]s tasks that the serial connection has been closed.
    ConnectionClosed,
}

/// Responsible for passing data and messages between the serial connection and tasks.
/// It uses the Actor model to maintain a single source for communicating between the
/// serial connection and tasks within the program.
///
/// It broadcasts [`SerialEvent`]s to worker tasks via a [`tokio::sync::broadcast`]
/// channel, and receives [`SerialMessage`]s from worker tasks via a [`tokio::sync::mpsc`]
/// channel.
pub struct SerialActor {
    connection: serial2_tokio::SerialPort,
    command_rx: tokio::sync::mpsc::Receiver<SerialMessage>,
    broadcast_channel: tokio::sync::broadcast::Sender<SerialEvent>,
}

impl SerialActor {
    /// Constructs a [`SerialActor`] Takes a serial port connection,
    /// receiver to a command channel, and a sender to a broadcast channel.
    pub fn new(
        connection: serial2_tokio::SerialPort,
        command_rx: tokio::sync::mpsc::Receiver<SerialMessage>,
        broadcast_channel: tokio::sync::broadcast::Sender<SerialEvent>,
    ) -> Self {
        Self {
            connection,
            command_rx,
            broadcast_channel,
        }
    }

    /// This is the heart and soul of the [`SerialActor`].
    /// `sericom` uses the Actor model to receive data from a serial connection
    /// and forward to other tasks for them to process. It also receives [`SerialEvent`]s
    /// from tasks and handles them accordingly; writes/sends data to the device
    /// over the serial connection and closes the connection when receiving
    /// [`SerialMessage::Shutdown`], ultimately causing the other tasks to shutdown.
    ///
    /// Since data is sent byte-by-byte over a serial connection, `run` will
    /// batch the data before sending it to other tasks to reduce the number of syscalls.
    pub async fn run(mut self) {
        let span = span!(Level::TRACE, "SerialActor");
        let _enter = span.enter();
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
                        Some(SerialMessage::SendBreak) => {
                            self.send_break().await;
                        }
                        None => break,
                    }
                }
                // Handle reading data from serial connection
                read_result = self.connection.read(&mut buffer) => {
                    // tracing::event!(Level::TRACE, "Data read");
                    match read_result {
                        Ok(0) => {
                            self.broadcast_channel.send(SerialEvent::ConnectionClosed).ok();
                            break;
                        }
                        Ok(n) => {
                            let data: std::sync::Arc<[u8]> = buffer[..n].into();
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

    async fn send_break(&mut self) {
        use tokio::time::{Duration, sleep};
        let _ = self.connection.set_break(true);
        sleep(Duration::from_millis(500)).await;
        let _ = self.connection.set_break(false);
    }
}
