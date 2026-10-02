#![warn(missing_debug_implementations)]
#![warn(unreachable_pub)]
#![deny(unused_must_use)]
#![allow(clippy::pub_use)]

pub mod builder;
pub mod config;
pub mod fmt;
pub mod prelude;
pub(crate) mod utils;
pub(crate) mod writer;

#[cfg(acta_wasm_blocking)]
compile_error!(
    "acta: the `file`, `compress`, `custom-async` and `native-async` features are not \
     supported on wasm32; build with `default-features = false` and enable `wasm-console`"
);

pub use builder::{TracingGuard, init};
#[cfg(feature = "file")]
pub use config::FileConfig;
pub use config::{
    ColorDepth, Config, ConfigBuilder, Filter, Format, Icons, LayerConfig, Level, LevelLabels,
    Rotation, Style, Theme, Writer, WriterTarget,
};
pub use fmt::Formatter;

pub use tracing::{
    debug, debug_span, error, error_span, info, info_span, trace, trace_span, warn, warn_span,
};

#[cfg(acta_async)]
pub use config::AsyncMode;
#[cfg(feature = "custom-async")]
pub use config::DEFAULT_ASYNC_BUFFER_SIZE;

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ActaError {
    #[error("invalid filter directive: {0}")]
    InvalidDirective(#[from] tracing_subscriber::filter::ParseError),
    #[error("failed to reload filter: {0}")]
    Reload(#[from] tracing_subscriber::reload::Error),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("failed to set global tracing subscriber: {0}")]
    SetGlobalDefault(#[from] tracing::subscriber::SetGlobalDefaultError),
    #[error(transparent)]
    LogTracer(#[from] tracing_log::log_tracer::SetLoggerError),
}

pub type Result<T> = std::result::Result<T, ActaError>;
