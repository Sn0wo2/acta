use std::io::{self, Write};

use tracing_appender::non_blocking::{NonBlocking, WorkerGuard};
use tracing_subscriber::fmt::MakeWriter;

use super::Stream;

pub(crate) struct NativeAsyncWriter {
    writer: NonBlocking,
    _guard: WorkerGuard,
}

impl NativeAsyncWriter {
    pub(crate) fn new(stream: Stream) -> Self {
        let sink: Box<dyn Write + Send> = match stream {
            Stream::Out => Box::new(io::stdout()),
            Stream::Err => Box::new(io::stderr()),
        };
        let (writer, _guard) = tracing_appender::non_blocking(sink);
        Self { writer, _guard }
    }
}

impl MakeWriter<'_> for NativeAsyncWriter {
    type Writer = NonBlocking;

    fn make_writer(&self) -> NonBlocking {
        self.writer.to_owned()
    }
}
