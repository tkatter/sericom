#![allow(unused)]

use std::{
    collections::HashMap,
    sync::{Arc, atomic::AtomicBool},
};

use tokio::task::JoinSet;

use crate::{
    cli::open_connection,
    screen::{Rect, ScreenBuffer},
    serial_actor::{SerialActor, SerialEvent, SerialMessage, tasks::run_stdin_input},
};

pub type SessionID = String;

pub struct SessionManager {
    pub(crate) active: Option<SessionID>,
    pub(crate) sessions: HashMap<SessionID, SessionHandle>,
}

pub struct SessionHandle {
    pub(crate) buffer: Arc<tokio::sync::RwLock<ScreenBuffer>>,
    /// Channel for communication from outside the session to the session
    pub(crate) tx: tokio::sync::mpsc::Sender<SerialMessage>,
    pub(crate) events: tokio::sync::broadcast::Receiver<SerialEvent>,
    /// A sessions async tasks
    pub(crate) tasks: JoinSet<()>,
    pub(crate) meta: SessionMeta,
}

pub struct SessionMeta {
    pub(crate) baud: u32,
}

impl SessionManager {
    pub fn new() -> Self {
        SessionManager {
            active: None,
            sessions: HashMap::new(),
        }
    }

    pub fn spawn<'a>(&mut self, port: &'a str, baud: u32) -> SessionID {
        match self.sessions.keys().find(|k| *k == port) {
            Some(session) => return session.clone(),
            None => {
                let (new_id, new_session);
                new_id = port.to_owned();
                new_session = SessionHandle::new(port, baud);

                self.sessions.insert(new_id.clone(), new_session);
                return new_id;
            }
        }
    }
}

impl SessionHandle {
    pub fn new<'a>(port: &'a str, baud: u32) -> Self {
        let (tx, rx) = tokio::sync::mpsc::channel::<SerialMessage>(100);
        let (events_tx, _) = tokio::sync::broadcast::channel::<SerialEvent>(128);

        let (session_events, parser_events);
        session_events = events_tx.subscribe();
        parser_events = events_tx.subscribe();

        let connection = open_connection(baud, port).unwrap();
        let (term_w, term_h) = crossterm::terminal::size().unwrap_or((80, 20));

        let sb = ScreenBuffer::new(Rect::new((0u16, 0u16).into(), term_w, term_h));
        let buffer = Arc::new(tokio::sync::RwLock::new(sb));
        let actor = SerialActor::new(connection, rx, events_tx);

        let mut tasks = JoinSet::new();
        tasks.spawn(actor.run());
        tasks.spawn(parse_task(Arc::clone(&buffer), parser_events));

        Self {
            buffer,
            tx,
            events: session_events,
            tasks,
            meta: SessionMeta { baud },
        }
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

                // Short-lived write lock
                {
                    let mut buf = buffer.write().await;
                    let mut driver = ScreenDriver::new(&mut buf);
                    driver.process_events(parsed);
                }
            }
            SerialEvent::Error(_) => todo!(),
            SerialEvent::ConnectionClosed => break,
        }
    }
}
