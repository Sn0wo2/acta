#![warn(missing_debug_implementations)]
#![warn(unreachable_pub)]
#![deny(unused_must_use)]
#![allow(clippy::pub_use)]

mod builder;
mod config;
mod fmt;
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
    ColorDepth, Config, ConfigBuilder, Format, Icons, JsonOptions, LevelAlignment, LevelLabels,
    PrettyOptions, Rotation, Style, Theme, Writer, WriterTarget,
};
pub use fmt::Formatter;

#[cfg(acta_async)]
pub use config::AsyncMode;

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
#[allow(variant_size_differences)]
pub enum ActaError {
    #[error("failed to reload filter: {0}")]
    Reload(#[from] tracing_subscriber::reload::Error),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("failed to set global tracing subscriber: {0}")]
    SetGlobalDefault(#[from] tracing::subscriber::SetGlobalDefaultError),
    #[error(transparent)]
    LogTracer(#[from] tracing_log::log_tracer::SetLoggerError),
    #[error("custom-async writers require an active tokio runtime")]
    RuntimeUnavailable,
}

pub type Result<T> = std::result::Result<T, ActaError>;
