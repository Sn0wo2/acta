use std::io::{self, Write};
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::io::{AsyncWrite, AsyncWriteExt};
use tokio::sync::mpsc;
use tracing_subscriber::fmt::MakeWriter;

use super::Stream;

#[derive(Debug)]
pub(crate) struct CustomAsyncWriter {
    sender: mpsc::Sender<Vec<u8>>,
    dropped: AtomicU64,
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
            dropped: AtomicU64::new(0),
        }
    }
}

impl Write for &CustomAsyncWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self.sender.try_reserve() {
            Ok(permit) => {
                permit.send(buf.to_vec());
                Ok(buf.len())
            }
            Err(mpsc::error::TrySendError::Full(())) => {
                let dropped = self.dropped.fetch_add(1, Ordering::Relaxed) + 1;
                if dropped == 1 || dropped.is_multiple_of(1024) {
                    let _unused = writeln!(
                        io::stderr(),
                        "acta: async writer buffer full ({}), {dropped} log messages dropped so far",
                        self.sender.max_capacity()
                    );
                }
                Ok(buf.len())
            }
            Err(mpsc::error::TrySendError::Closed(())) => Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "async writer closed",
            )),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl<'a> MakeWriter<'a> for CustomAsyncWriter {
    type Writer = &'a Self;

    fn make_writer(&'a self) -> Self::Writer {
        self
    }
}
