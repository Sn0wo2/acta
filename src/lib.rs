#![warn(missing_debug_implementations)]
#![warn(unreachable_pub)]
#![deny(unused_must_use)]
#![allow(clippy::pub_use)]

mod builder;
mod config;
mod event;
mod fmt;
mod span;
pub(crate) mod writer;

#[cfg(acta_wasm_blocking)]
compile_error!(
    "acta: the `file`, `compress`, `custom-async` and `native-async` features are not \
     supported on wasm32; build with `default-features = false` and enable `wasm-console`"
);

pub use builder::{Logger, TracingGuard, init};
#[cfg(feature = "file")]
pub use config::FileConfig;
pub use config::{
    ColorDepth, Config, ConfigBuilder, Format, Icons, JsonOptions, LevelAlignment, LevelLabels,
    PrettyOptions, Rotation, Style, Theme, Writer, WriterTarget,
};
pub use event::Event;
pub use fmt::Formatter;
pub use writer::buffered::{BufferedWriteGuard, BufferedWriter};

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
    #[error("event metadata schema limit reached (maximum 4096 schemas)")]
    EventSchemaLimit,
    #[error("event metadata schemas exceed the 4 MiB registry budget")]
    EventSchemaBudget,
    #[error("event field name `{0}` is reserved for the event message")]
    EventReservedField(String),
    #[error("failed to allocate event metadata registry storage")]
    EventSchemaAllocation,
    #[error("span operation failed: {0}")]
    Span(String),
}

pub type Result<T> = std::result::Result<T, ActaError>;
