use std::io::{self, Write};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::io::{AsyncWrite, AsyncWriteExt};
use tokio::sync::mpsc;
use tracing_subscriber::fmt::MakeWriter;

use super::Stream;

#[derive(Clone, Debug)]
pub(crate) struct CustomAsyncWriter {
    sender: mpsc::Sender<Vec<u8>>,
    dropped: Arc<AtomicU64>,
}

impl CustomAsyncWriter {
    pub(crate) fn new(stream: Stream, capacity: usize) -> Self {
        let (sender, mut receiver) = mpsc::channel::<Vec<u8>>(capacity);

        tokio::spawn(async move {
            let mut writer: Box<dyn AsyncWrite + Unpin + Send> = match stream {
                Stream::Out => Box::new(tokio::io::stdout()),
                Stream::Err => Box::new(tokio::io::stderr()),
            };
            while let Some(data) = receiver.recv().await {
                if let Err(e) = writer.write_all(&data).await {
                    let _unused = writeln!(io::stderr(), "async writer error: {e}");
                }
            }
        });

        Self {
            sender,
            dropped: Arc::new(AtomicU64::new(0)),
        }
    }
}

impl Write for CustomAsyncWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self.sender.try_send(buf.to_vec()) {
            Ok(_) => Ok(buf.len()),
            Err(mpsc::error::TrySendError::Full(_)) => {
                let dropped = self.dropped.fetch_add(1, Ordering::Relaxed) + 1;
                if dropped == 1 || dropped % 1024 == 0 {
                    let _unused = writeln!(
                        io::stderr(),
                        "acta: async writer buffer full ({}), {dropped} log messages dropped so far",
                        self.sender.max_capacity()
                    );
                }
                Ok(buf.len())
            }
            Err(mpsc::error::TrySendError::Closed(_)) => Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "async writer closed",
            )),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl MakeWriter<'_> for CustomAsyncWriter {
    type Writer = Self;

    fn make_writer(&self) -> Self {
        self.clone()
    }
}
