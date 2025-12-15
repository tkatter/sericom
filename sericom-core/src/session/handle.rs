use miette::Context;
use std::{path::PathBuf, sync::Arc};
use tokio::{io::AsyncWriteExt, sync::oneshot, task::JoinSet};
use tracing::{Instrument, Span, debug, error, info, trace, warn};

use crate::{
    SeriError,
    cli::open_connection,
    compat_port_path,
    configs::get_config,
    create_recursive,
    screen::{Rect, ScreenBuffer},
    serial_actor::{SerialActor, SerialEvent, SerialMessage},
    with_help,
};

/// A handle to a session - used to manage the session in headless/interactive modes.
#[derive(Debug)]
pub struct SessionHandle {
    /// The Session's dedicated [`ScreenBuffer`].
    pub(crate) buffer: Arc<tokio::sync::RwLock<ScreenBuffer>>,
    /// Channel for communication from outside the session to the session.
    ///
    /// A [`Sender`] so that the [`SessionHandle`] can send [`Write`], [`SendBreak`],
    /// and [`Shutdown`] events to the session's [`SerialActor`].
    ///
    /// [`Sender`]: tokio::sync::mpsc::Sender
    /// [`Write`]: crate::serial_actor::SerialMessage::Write
    /// [`SendBreak`]: crate::serial_actor::SerialMessage::SendBreak
    /// [`Shutdown`]: crate::serial_actor::SerialMessage::Shutdown
    pub(crate) tx: tokio::sync::mpsc::Sender<SerialMessage>,
    /// A [`Receiver`] of the session's [`SerialEvent`]s.
    ///
    /// Intended for the [`SessionManager`] to recieve these events based on the
    /// session's state and where it's received data is being written to handle accordingly.
    ///
    /// [`Receiver`]: tokio::sync::broadcast::Receiver
    /// [`SessionManager`]: super::SessionManager
    pub(crate) events_tx: tokio::sync::broadcast::Sender<SerialEvent>,
    /// A sessions async tasks.
    ///
    /// Currently a session only has two tasks, the [`SerialActor::run`] and
    /// the task responsible for parsing the session's incoming data/byte stream.
    /// The [`JoinSet`] is used so that when a session is terminated, all tasks
    /// associated with a session can be gracefully killed together.
    pub(crate) tasks: JoinSet<crate::Result<()>>,
}

async fn instrument_task<F>(
    fut: F,
    span: Option<tracing::Span>,
    events: tokio::sync::broadcast::Sender<SerialEvent>,
) -> crate::Result<()>
where
    F: Future<Output = crate::Result<()>> + Send + 'static,
{
    let res = if let Some(span) = span {
        fut.instrument(span).await
    } else {
        fut.await
    };

    if let Err(ref e) = res {
        let _ = events.send(SerialEvent::Error(e.clone()));
    }
    res
}

impl SessionHandle {
    /// Spawn a new session and optionally stream to a `file` or run in headless mode.
    pub fn spawn(
        meta: &super::SessionMeta,
        with_file: Option<Option<PathBuf>>,
        headless: bool,
        mgr_tx: oneshot::Sender<super::SeriError>,
    ) -> miette::Result<Self> {
        let (tx, rx) = tokio::sync::mpsc::channel::<SerialMessage>(100);
        let (events_tx, _) = tokio::sync::broadcast::channel::<SerialEvent>(128);

        let connection = open_connection(meta.baud, &meta.port)
            .map_err(|e| with_help!(e, "Is the port already open? Do you have permission? Typo?"))
            .wrap_err(format!("Failed to open port: '{}'", meta.port.display()))?;
        let (term_w, term_h) = if headless {
            #[cfg(not(test))]
            {
                (80, 24)
            }
            #[cfg(test)]
            {
                (120, 10)
            }
        } else {
            crossterm::terminal::size().unwrap_or((80, 24))
        };

        let sb = ScreenBuffer::new(Rect::new((0u16, 0u16).into(), term_w, term_h));
        let buffer = Arc::new(tokio::sync::RwLock::new(sb));
        let actor = SerialActor::new(connection, rx, events_tx.clone());

        let span = {
            let info = tracing::info_span!("session");
            let dbg = tracing::debug_span!("session", port = %meta.port.display());
            if dbg.is_disabled() { info } else { dbg }
        };
        let _enter = span.enter();
        debug!(port=%meta.port.display(), baud=%meta.baud, "opened connection");

        let mut handle = Self {
            buffer,
            tx,
            events_tx,
            tasks: JoinSet::new(),
        };
        handle.spawn_monitor_task(mgr_tx);
        handle.spawn_actor_task(actor);
        handle.spawn_parse_task();

        if with_file.is_some() {
            let config = get_config()?;
            let default_out_dir = PathBuf::from(&config.defaults.out_dir);
            let file_path = if let Some(Some(path)) = with_file {
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
            } else {
                let default_out_dir = PathBuf::from(&config.defaults.out_dir);
                drop(config);
                compat_port_path!(default_out_dir, &meta.port)
            };
            handle.spawn_file_task(file_path);
        }

        Ok(handle)
    }

    /// Shutdown this session.
    ///
    /// Signals associated tasks to shutdown and waits for them to complete.
    pub async fn shutdown(self) {
        if let Err(e) = self.tx.send(SerialMessage::Shutdown).await {
            warn!("error sending shutdown to actor: {e}");
            let mut tasks = self.tasks;
            tasks.abort_all();
        } else {
            self.tasks.join_all().await;
        }
        trace!("all tasks finished, shutting down");
    }

    /// Monitors the tasks' broadcast channel for ones that send an error.
    ///
    /// Upon receiving an error, propogates to the [`SessionManager`] to handle.
    ///
    /// [`SessionManager`]: super::SessionManager
    fn spawn_monitor_task(&mut self, mgr_tx: oneshot::Sender<super::SeriError>) {
        let (tasks_tx, mut mon_rx) = (self.tx.clone(), self.events_tx.subscribe());
        let span = tracing::Span::current();
        self.tasks.spawn(
            async move {
                while let Ok(event) = mon_rx.recv().await {
                    match event {
                        SerialEvent::Error(e) => {
                            error!(err=%e, "task failed");
                            let _ = mgr_tx.send(e);
                            let _ = tasks_tx.send(SerialMessage::Shutdown).await;
                            break; // so compiler doesn't complain about mgr_tx being moved
                        }
                        SerialEvent::ConnectionClosed => break,
                        _ => {}
                    }
                }

                Ok(())
            }
            .instrument(span),
        );
    }

    /// Spawns a task that runs the [`SerialActor`] for the session.
    fn spawn_actor_task(&mut self, actor: SerialActor) {
        self.tasks.spawn(instrument_task(
            actor.run(),
            Some(Span::current()),
            self.events_tx.clone(),
        ));
    }

    /// Spawns a task that writes the session's output to a file.
    fn spawn_file_task(&mut self, f_path: PathBuf) {
        self.tasks.spawn(instrument_task(
            file_task(f_path, Arc::clone(&self.buffer), self.events_tx.subscribe()),
            Some(Span::current()),
            self.events_tx.clone(),
        ));
    }

    /// Spawn the parsing task.
    ///
    /// This task is responsible for parsing the session's incoming data and
    /// writing it to the sessions [`ScreenBuffer`].
    fn spawn_parse_task(&mut self) {
        self.tasks.spawn(instrument_task(
            parse_task(
                Arc::clone(&self.buffer),
                self.events_tx.clone(),
                self.events_tx.subscribe(),
            ),
            Some(Span::current()),
            self.events_tx.clone(),
        ));
    }
}

#[tracing::instrument(skip_all)]
async fn file_task(
    f_path: PathBuf,
    buffer: Arc<tokio::sync::RwLock<ScreenBuffer>>,
    mut events_rx: tokio::sync::broadcast::Receiver<SerialEvent>,
) -> crate::Result<()> {
    let mut file = tokio::fs::File::create(&f_path)
        .await
        .inspect_err(|_| {
            error!("failed to create file: {}", f_path.display());
        })
        .map_err(|e| {
            SeriError::from(e).add_ctx(format!("Failed to create file: {}", f_path.display()))
        })?;

    info!(file=%f_path.display(), "created file");
    let mut last_idx = buffer.read().await.view_start;

    while let Ok(event) = events_rx.recv().await {
        match event {
            SerialEvent::LinesWritten(0) => {}
            SerialEvent::LinesWritten(1) => {
                trace!("LinesWritten=1");
                let sb = buffer.read().await;
                if sb.view_start.saturating_sub(1) == last_idx {
                    continue;
                }
                last_idx += 1;

                trace!(%last_idx, view_start=%sb.view_start);

                if let Some(line) = sb.lines.get(last_idx as usize) {
                    file.write_all(&line.ascii_bytes().collect::<Vec<u8>>())
                        .await
                        .map_err(|e| {
                            SeriError::from(e).add_ctx("Failed to write to file".into())
                        })?;
                }
                drop(sb);
            }
            SerialEvent::LinesWritten(num) => {
                trace!("LinesWritten={num}");
                let sb = buffer.read().await;
                let last_line_idx = sb.view_start.saturating_sub(1);
                let range = {
                    let end = if last_idx + num > last_line_idx {
                        last_line_idx
                    } else {
                        last_idx + num
                    };

                    let r = if last_idx == 0 {
                        last_idx..=end
                    } else {
                        last_idx + 1..=end
                    };

                    trace!(
                        %last_idx,
                        view_start=%sb.view_start,
                        %last_line_idx,
                        ?r
                    );

                    last_idx += num;
                    r
                };

                for idx in range {
                    if let Some(line) = sb.lines.get(idx as usize) {
                        file.write_all(&line.ascii_bytes().collect::<Vec<u8>>())
                            .await
                            .map_err(|e| {
                                SeriError::from(e).add_ctx("Failed to write to file".into())
                            })?;
                    }
                }
            }
            SerialEvent::ConnectionClosed => {
                let sb = buffer.read().await;
                let viewport = sb.buff_rect();
                trace!(range=?sb.view_start..=(viewport.height + sb.view_start), "flushing");
                for idx in sb.view_start..(viewport.height + sb.view_start) {
                    if let Some(line) = sb.lines.get(idx as usize) {
                        file.write_all(&line.ascii_bytes().collect::<Vec<u8>>())
                            .await
                            .map_err(|e| {
                                SeriError::from(e).add_ctx("Failed to write to file".into())
                            })?;
                    }
                }
                file.flush()
                    .await
                    .map_err(|e| SeriError::from(e).add_ctx("Failed to flush to file".into()))?;
                trace!("connection closed");
                break;
            }
            _ => {}
        }
    }
    Ok(())
}

#[tracing::instrument(skip_all, level = "debug")]
async fn parse_task(
    buffer: Arc<tokio::sync::RwLock<ScreenBuffer>>,
    events_tx: tokio::sync::broadcast::Sender<SerialEvent>,
    mut events_rx: tokio::sync::broadcast::Receiver<SerialEvent>,
) -> crate::Result<()> {
    use crate::screen::{ByteParser, ScreenDriver};

    let mut parser = ByteParser::new();
    while let Ok(event) = events_rx.recv().await {
        match event {
            SerialEvent::Data(bytes) => {
                trace!("received {} bytes", bytes.len());
                let parsed = parser.feed(&bytes);
                let mut buf = buffer.write().await;

                let num_lines = ScreenDriver::new(&mut buf).process_events(parsed);
                drop(buf);
                let _ = events_tx.send(SerialEvent::LinesWritten(num_lines));
            }
            SerialEvent::ConnectionClosed => {
                trace!("connection closed");
                break;
            }
            _ => {}
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{
        env::{current_dir, temp_dir},
        fs::{File, remove_file},
        io::{BufRead, BufReader, Write},
        path::PathBuf,
        time::Duration,
    };
    use tokio::time::sleep;

    use super::super::*;
    use test_sericom::get_pts_pair;

    #[tokio::test]
    #[test_log::test]
    async fn write_a_file() {
        crate::configs::init_for_tests();

        let bytes = include_bytes!("./mod.rs");
        let tmp_dir = temp_dir();
        let tmp_path = tmp_dir.join("sericom_core_test_write_a_file.txt");
        let og_path = current_dir().unwrap().join("src/session/mod.rs");

        let mut manager = SessionManager::new();
        let (mut master, slave) = get_pts_pair().unwrap();

        let id = manager
            .spawn(
                PathBuf::from(slave),
                9600,
                Some(Some(tmp_path.clone())),
                true,
            )
            .unwrap();

        sleep(Duration::from_millis(500)).await;
        master.write_all(bytes).unwrap();
        sleep(Duration::from_millis(500)).await;

        manager.kill(id).await;

        let out_file = BufReader::new(File::open(&tmp_path).unwrap());
        let og_file = BufReader::new(File::open(&og_path).unwrap());
        let out_lines = out_file.lines().map(Result::unwrap);
        let og_lines = og_file.lines().map(Result::unwrap);

        for (og_line, out_line) in og_lines.zip(out_lines) {
            assert_eq!(og_line, out_line);
        }

        let _ = remove_file(tmp_path);
    }
}
