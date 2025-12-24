use miette::Diagnostic;
use thiserror::Error;

#[derive(Debug, Clone, Diagnostic, Error)]
pub enum SeriError {
    #[error("Parsing error: {0:?}")]
    Parsing(String),
    #[error("Invalid session id: {0}")]
    Session(super::SessionID),
    #[error("Task error: {0:?}")]
    TaskError(String),
    #[error("{}", ctx.as_ref().map_or("I/O Error", |c| c))]
    Io {
        #[source]
        source: std::sync::Arc<std::io::Error>,
        #[help]
        help: Option<String>,
        ctx: Option<String>,
    },
    #[error("Serial connection error")]
    Connection {
        #[source]
        source: std::sync::Arc<std::io::Error>,
        #[help]
        help: Option<String>,
    },
    #[error("Serial connection error")]
    SendErr(#[from] tokio::sync::mpsc::error::SendError<crate::serial_actor::SerialMessage>),
}

#[allow(unused)]
impl SeriError {
    #[must_use]
    pub fn add_ctx(mut self, msg: String) -> Self {
        match &mut self {
            Self::TaskError(_) => todo!(),
            Self::Io { source, help, ctx } => *ctx = Some(msg),
            Self::Connection { source, help } => todo!(),
            Self::SendErr(_) => todo!(),
            Self::Session(_) => todo!(),
            Self::Parsing(_) => todo!(),
        }

        self
    }

    #[must_use]
    pub fn add_help(mut self, msg: String) -> Self {
        match &mut self {
            Self::TaskError(_) => todo!(),
            Self::Io { source, help, ctx } => *help = Some(msg),
            Self::Connection { source, help } => *help = Some(msg),
            Self::SendErr(_) => todo!(),
            Self::Session(_) => todo!(),
            Self::Parsing(_) => todo!(),
        }

        self
    }
}

impl From<std::io::Error> for SeriError {
    fn from(err: std::io::Error) -> Self {
        Self::Io {
            source: std::sync::Arc::new(err),
            help: None,
            ctx: None,
        }
    }
}

#[macro_export]
macro_rules! err_into {
    ($err:expr, $variant:path) => {{
        $variant {
            source: std::sync::Arc::new($err),
            help: None,
        }
    }};

    ($err:expr, $variant:path, help = $help:expr) => {{
        $variant {
            source: $err,
            help: $help,
        }
    }};

    ($err:expr, $variant:path, help = $help:expr) => {{
        $variant {
            source: $err,
            help: $help,
        }
    }};
}

#[macro_export]
macro_rules! with_help {
    ($err:expr, $msg:literal) => {{
        match $err {
            $crate::SeriError::Io { source, .. } => {
                $crate::SeriError::Io {
                    source,
                    help: Some($msg.into()),
                    ctx: None,
                }
            }
            $crate::SeriError::Connection { source, .. } => {
                $crate::SeriError::Connection {
                    source,
                    help: Some($msg.into()),
                }
            }
            _ => $err,
        }
    }};
    ($err:expr, $msg:literal , $($args:expr),* ) => {{
        match $err {
            $crate::SeriError::Io { source, .. } => {
                $crate::SeriError::Io {
                    source,
                    help: Some(format!($msg, $($args),*)),
                    ctx: None,
                }
            }
            $crate::SeriError::Connection { source, .. } => {
                $crate::SeriError::Connection {
                    source,
                    help: Some(format!($msg, $($args),*)),
                }
            }
            _ => $err,
        }
    }};
    ($var:path [ $err:expr ] : $msg:literal , $($args:expr),*) => {{
        $var {
            source: $err,
            help: Some(format!($msg, $($args),*)),
        }
    }};

    ($var:path [ $err:expr ] : $msg:literal $($more_msg:literal)* , $($args:expr),*) => {{
        $var {
            source: $err,
            help: Some(format!(concat!($msg, $("\n", $more_msg),+), $($args),*)),
        }
    }};
}

#[test]
fn test_macro() {
    fn err() -> miette::Result<()> {
        let e = std::io::Error::last_os_error();
        // let err: SeriError = with_help!(
        //     SeriError::ConnectionIo[e]: "hello {} {} {}"
        //     "this is another line {}"
        //     "and another line {}",
        //     "world", 1, 2, 3, 4
        // );
        // let err: SeriError = with_help!(
        //     SeriError::ConnectionIo[e]: "hello {} {} {}", 1, 2, 3
        // );
        let err = SeriError::Io {
            source: std::sync::Arc::new(e),
            help: None,
            ctx: None,
        };

        Err(err).map_err(|e| with_help!(e, "hello {} {} {}", 1, 2, 3).into())
    }

    err().inspect_err(|err| eprintln!("{err:?}")).ok();
}
