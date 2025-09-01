use std::ops::Range;

use crossterm::style::Stylize;
use miette::{NamedSource, SourceSpan};

/// A wrapper around error types that may arise from attempting to parse a config
/// file.
///
/// Used to allow better, more specific, handling of errors that may arise
/// from parsing the file.
///
/// [`ConfigError::AlreadyInitialized`] should theorhetically
/// never arise; however, in the situation where
/// [`initialize_config()`][`super::initialize_config()`] were
/// called and `static CONFIG` is already constructed - [`ConfigError::AlreadyInitialized`]
/// would be the error.
#[derive(Debug, miette::Diagnostic, thiserror::Error)]
pub enum ConfigError {
    #[error(transparent)]
    IoError(#[from] std::io::Error),
    #[error(transparent)]
    #[diagnostic(transparent)]
    TomlError(#[from] TomlError),
    #[error(
        "Config already initialized.\nPlease report the bug to {}", "https://github.com/tkatter/sericom".bold()
    )]
    AlreadyInitialized,
}

/// A wrapper around [`toml::de::Error`] to print custom error messages with [`miette`].
#[derive(thiserror::Error, miette::Diagnostic, Debug)]
#[error("{}", "Error reading config file".red())]
#[diagnostic(
    help("{}", self.msg.split_once(',').unwrap_or(("", self.msg.as_str())).1.trim())
)]
pub struct TomlError {
    #[label("{}", self.msg.split_once(',').unwrap_or((self.msg.as_str(), "")).0.trim())]
    at: SourceSpan,

    #[source_code]
    src: NamedSource<String>,

    msg: String,
}

impl TomlError {
    pub(crate) fn new(span: Range<usize>, source: String, message: String) -> Self {
        let span_len = span.end - span.start;
        let at: SourceSpan = (span.start, span_len).into();
        let src = NamedSource::new("config.toml", source);
        let msg = message;
        Self { at, src, msg }
    }
}
