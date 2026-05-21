use std::io::Write;
use tokio::io::{AsyncWrite, AsyncWriteExt, stderr, stdout};
use tokio::sync::mpsc;
use tracing_subscriber::fmt::MakeWriter;

use super::AsyncWriterTarget;

#[derive(Clone, Debug)]
pub struct AsyncWriter {
    sender: mpsc::Sender<Vec<u8>>,
}

impl Write for AsyncWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self.sender.try_send(buf.to_vec()) {
            Ok(_) => Ok(buf.len()),
            Err(mpsc::error::TrySendError::Full(_)) => {
                let _unused = writeln!(
                    std::io::stderr(),
                    "acta: async writer buffer full (4096), dropping log message"
                );
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

pub fn async_writer_for(target: AsyncWriterTarget) -> AsyncWriter {
    let (sender, mut receiver) = mpsc::channel::<Vec<u8>>(4096);

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

    AsyncWriter { sender }
}
