use std::io::Write;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::io::{AsyncWrite, AsyncWriteExt, stderr, stdout};
use tokio::sync::mpsc;
use tracing_subscriber::fmt::MakeWriter;

use super::AsyncWriterTarget;

/// Per-message warnings would hammer stderr exactly when the writer is saturated.
const DROP_WARN_INTERVAL: u64 = 1024;

#[derive(Clone, Debug)]
pub struct AsyncWriter {
    sender: mpsc::Sender<Vec<u8>>,
    dropped: Arc<AtomicU64>,
}

impl Write for AsyncWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self.sender.try_send(buf.to_vec()) {
            Ok(_) => Ok(buf.len()),
            Err(mpsc::error::TrySendError::Full(_)) => {
                let dropped = self.dropped.fetch_add(1, Ordering::Relaxed) + 1;
                if dropped == 1 || dropped % DROP_WARN_INTERVAL == 0 {
                    let _unused = writeln!(
                        std::io::stderr(),
                        "acta: async writer buffer full ({}), {dropped} log messages dropped so far",
                        self.sender.max_capacity()
                    );
                }
                Ok(buf.len())
            }
            Err(mpsc::error::TrySendError::Closed(_)) => Err(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                "async writer closed",
            )),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl MakeWriter<'_> for AsyncWriter {
    type Writer = Self;

    fn make_writer(&self) -> Self::Writer {
        self.clone()
    }
}

/// Spawn a Tokio task that drains a bounded channel into `target`, returning a
/// writer whose channel holds up to `capacity` queued log messages before new
/// ones are dropped.
pub fn async_writer_for(target: AsyncWriterTarget, capacity: usize) -> AsyncWriter {
    let (sender, mut receiver) = mpsc::channel::<Vec<u8>>(capacity);

    tokio::spawn(async move {
        let writer: &mut (dyn AsyncWrite + Unpin + Send) = match target {
            AsyncWriterTarget::Stdout => &mut stdout(),
            AsyncWriterTarget::Stderr => &mut stderr(),
        };

        while let Some(data) = receiver.recv().await {
            if let Err(e) = writer.write_all(&data).await {
                let _unused = writeln!(std::io::stderr(), "async writer error: {e}");
            }
        }
    });

    AsyncWriter {
        sender,
        dropped: Arc::new(AtomicU64::new(0)),
    }
}
