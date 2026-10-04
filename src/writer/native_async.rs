use std::io::{self, Write};

use tracing_appender::non_blocking::{NonBlocking, WorkerGuard};

use super::Stream;

pub(crate) fn non_blocking(stream: Stream) -> (NonBlocking, WorkerGuard) {
    let sink: Box<dyn Write + Send> = match stream {
        Stream::Out => Box::new(io::stdout()),
        Stream::Err => Box::new(io::stderr()),
    };
    tracing_appender::non_blocking(sink)
}
