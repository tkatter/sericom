//! This module holds all of the code directly responsible for interacting
//! with the serial connection and tasks within the program.

pub mod tasks;

use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

use crate::SeriError;

/// Represents messages/commands that are sent from worker tasks to the [`SerialActor`] to process.
#[non_exhaustive]
#[derive(Debug, Clone)]
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
    Error(SeriError),
    /// Tells the [`SerialActor`]s tasks that the serial connection has been closed.
    ///
    /// This serves a different purpose from [`SerialMessage::Shutdown`] where
    /// [`SerialMessage::Shutdown`] is mean to instruct the [`SerialActor`] to
    /// shutdown the connection. `ConnectionClosed` is used for the [`SerialActor`]
    /// to broadcast to listeners that the connection has been shutdown by the device.
    ConnectionClosed,
    LinesWritten(u32),
}

/// Responsible for passing data and messages between the serial connection and tasks.
///
/// Uses the Actor model to maintain a single source for communicating between the
/// serial connection and tasks within the program.
///
/// It broadcasts [`SerialEvent`]s to worker tasks via a [`tokio::sync::broadcast`]
/// channel, and receives [`SerialMessage`]s from worker tasks via a [`tokio::sync::mpsc`]
/// channel.
pub struct SerialActor<S> {
    connection: S,
    // connection: serial2_tokio::SerialPort,
    command_rx: tokio::sync::mpsc::Receiver<SerialMessage>,
    tasks_broadcast: tokio::sync::broadcast::Sender<SerialEvent>,
}

impl<S> SerialActor<S>
where
    S: AsyncRead + AsyncWrite + AsyncWriteExt + Unpin + Send + 'static,
{
    /// Constructs a [`SerialActor`] Takes a serial port connection,
    /// receiver to a command channel, and a sender to a broadcast channel.
    #[must_use]
    pub const fn new(
        // pub const fn new(
        // connection: serial2_tokio::SerialPort,
        connection: S,
        command_rx: tokio::sync::mpsc::Receiver<SerialMessage>,
        tasks_broadcast: tokio::sync::broadcast::Sender<SerialEvent>,
    ) -> Self {
        Self {
            connection,
            command_rx,
            tasks_broadcast,
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
    #[tracing::instrument(skip_all, level = "debug")]
    pub async fn run(mut self) -> crate::Result<()> {
        use tracing::{debug, error, trace};

        trace!("running SerialActor");
        let mut buffer = vec![0u8; 2048];
        loop {
            tokio::select! {
                // Handle commands/input from tasks
                cmd = self.command_rx.recv() => {
                    trace!(?cmd, "command_rx.recv()");
                    match cmd {
                        Some(SerialMessage::Write(data)) => {
                            if let Err(e) = self.connection.write_all(&data).await {
                                error!("error writing to connection: {e}");
                                return Err(crate::SeriError::from(e).add_ctx("Error writing to the connection".into()));
                            }
                        }
                        Some(SerialMessage::Shutdown) => {
                            debug!("recieved shutdown command");
                            self.tasks_broadcast.send(SerialEvent::ConnectionClosed).ok();
                            return Ok(());
                        }
                        Some(SerialMessage::SendBreak) => {
                            debug!("sending break signal");
                            // self.send_break().await;
                        }
                        None => {
                            trace!("command_rx.recv() was None");
                            return Err(SeriError::TaskError("No active tasks, shutting down.".into()));
                        }
                    }
                }
                // Handle reading data from serial connection
                read_result = self.connection.read(&mut buffer) => {
                    match read_result {
                        Ok(0) => {
                            debug!("closing connection - no bytes read");
                            self.tasks_broadcast.send(SerialEvent::ConnectionClosed).ok();
                            return Ok(());
                        }
                        Ok(n) => {
                            let data: std::sync::Arc<[u8]> = buffer[..n].into();
                            trace!("recieved {} bytes", data.len());
                            self.tasks_broadcast.send(SerialEvent::Data(data)).ok();
                        }
                        Err(e) => {
                            error!("error reading from connection: {e}");
                            return Err(SeriError::from(e).add_ctx("Error reading from connection".into()));
                        }
                    }
                }
            }
        }
    }
}

impl SerialActor<serial2_tokio::SerialPort> {
    async fn send_break(&self) {
        use tokio::time::{Duration, sleep};
        let _ = self.connection.set_break(true);
        sleep(Duration::from_millis(500)).await;
        let _ = self.connection.set_break(false);
    }
}
