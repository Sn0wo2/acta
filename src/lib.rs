#![warn(missing_debug_implementations)]
#![warn(unreachable_pub)]
#![deny(unused_must_use)]
#![allow(clippy::pub_use)]

pub mod builder;
pub mod color;
pub mod config;
pub mod fmt;
pub mod prelude;
#[cfg(any(feature = "file", feature = "custom-async", feature = "native-async"))]
pub mod writer;

pub use builder::{TracingGuard, build_layer, init};
pub use config::{
    ColorDepth, Config, Filter, Format, Icons, LayerConfig, Level, LevelLabels, Rotation, Style,
    Theme, Writer, WriterTarget,
};
pub use fmt::Formatter;

pub use tracing::{
    debug, debug_span, error, error_span, info, info_span, trace, trace_span, warn, warn_span,
};

#[cfg(feature = "custom-async")]
pub use writer::{AsyncWriter, async_writer_for};

#[cfg(any(feature = "custom-async", feature = "native-async"))]
pub use config::AsyncMode;

#[cfg(any(feature = "custom-async", feature = "native-async"))]
pub use writer::AsyncWriterTarget;

#[cfg(feature = "file")]
pub use crate::writer::{LogHandle, resolve_log_path, rotate_log_file};

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ActaError {
    #[error("log filter state lock poisoned")]
    LockPoisoned,
    #[error("invalid filter directive: {0}")]
    InvalidDirective(#[from] tracing_subscriber::filter::ParseError),
    #[error("failed to reload filter: {0}")]
    Reload(#[from] tracing_subscriber::reload::Error),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("failed to set global tracing subscriber: {0}")]
    SetGlobalDefault(#[from] tracing::subscriber::SetGlobalDefaultError),
}

pub type Result<T> = std::result::Result<T, ActaError>;

#[cfg(test)]
mod test;
