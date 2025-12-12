use std::sync::Arc;
use tokio::task::JoinSet;
use tracing::Instrument;

use crate::{
    cli::open_connection,
    screen::{Rect, ScreenBuffer},
    serial_actor::{SerialActor, SerialEvent, SerialMessage},
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
    pub(crate) events: tokio::sync::broadcast::Receiver<SerialEvent>,
    /// A sessions async tasks.
    ///
    /// Currently a session only has two tasks, the [`SerialActor::run`] and
    /// the task responsible for parsing the session's incoming data/byte stream.
    /// The [`JoinSet`] is used so that when a session is terminated, all tasks
    /// associated with a session can be gracefully killed together.
    pub(crate) tasks: JoinSet<()>,
}

impl SessionHandle {
    /// Spawn a new session for `port` with `baud`.
    #[allow(clippy::result_unit_err)]
    pub fn spawn(meta: &super::SessionMeta) -> miette::Result<Self> {
        let (tx, rx) = tokio::sync::mpsc::channel::<SerialMessage>(100);
        let (events_tx, _) = tokio::sync::broadcast::channel::<SerialEvent>(128);

        let session_events = events_tx.subscribe();
        let parser_events = events_tx.subscribe();

        let connection = open_connection(meta.baud, &meta.port)?; // TODO: HANDLE ERROR
        let (term_w, term_h) = crossterm::terminal::size().unwrap_or((80, 20));

        let sb = ScreenBuffer::new(Rect::new((0u16, 0u16).into(), term_w, term_h));
        let buffer = Arc::new(tokio::sync::RwLock::new(sb));
        let actor = SerialActor::new(connection, rx, events_tx);

        // Not storing the span in SessionHandle because I'm not sure if I'll
        // ever need to emit events outside of the SessionHandle in the same span.
        // If I do I could just re-create the span from SessionMeta.
        let span = tracing::info_span!("session", port = %meta.port, baud = %meta.baud);
        let _enter = span.enter();
        tracing::info!(target: "session", "Opened connection");

        let mut tasks = JoinSet::new();
        tasks.spawn(actor.run().instrument(span.clone()));
        tasks.spawn(parse_task(Arc::clone(&buffer), parser_events).instrument(span.clone()));

        Ok(Self {
            buffer,
            tx,
            events: session_events,
            tasks,
        })
    }

    pub async fn shutdown(self) {
        let _ = self.tx.send(SerialMessage::Shutdown).await;
        self.tasks.join_all().await;
    }
}

async fn parse_task(
    buffer: Arc<tokio::sync::RwLock<ScreenBuffer>>,
    mut events: tokio::sync::broadcast::Receiver<SerialEvent>,
) {
    use crate::screen::{ByteParser, ScreenDriver};

    let mut parser = ByteParser::new();

    while let Ok(event) = events.recv().await {
        match event {
            SerialEvent::Data(bytes) => {
                let parsed = parser.feed(&bytes);

                let mut buf = buffer.write().await;
                ScreenDriver::new(&mut buf).process_events(parsed);
            }
            SerialEvent::ConnectionClosed => {
                tracing::info!(target: "session::parse_task", "Connection closed");
                break;
            }
            _ => unreachable!("SerialActor never sends a SerialEvent::Error"),
        }
    }
}

/*
#[test]
fn sizes() {
    eprintln!(
        "size of handle: {}\n
        align of handle: {}\n
        size of buffer: {}\n
        align of buffer: {}\n
        size of tx: {}\n
        align of tx: {}\n
        size of events: {}\n
        align of events: {}\n
        size of tasks: {}\n
        align of tasks: {}\n
        size of Span: {}\n
        align of Span: {}",
        size_of::<SessionHandle>(),
        align_of::<SessionHandle>(),
        size_of::<Arc<tokio::sync::RwLock<ScreenBuffer>>>(),
        align_of::<Arc<tokio::sync::RwLock<ScreenBuffer>>>(),
        size_of::<tokio::sync::mpsc::Sender<SerialMessage>>(),
        align_of::<tokio::sync::mpsc::Sender<SerialMessage>>(),
        size_of::<tokio::sync::broadcast::Receiver<SerialEvent>>(),
        align_of::<tokio::sync::broadcast::Receiver<SerialEvent>>(),
        size_of::<JoinSet<()>>(),
        align_of::<JoinSet<()>>(),
        size_of::<tracing::Span>(),
        align_of::<tracing::Span>(),
    );
    assert!(1 > 2);
}
*/
