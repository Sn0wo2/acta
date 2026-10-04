use std::io::{self, Write};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex, PoisonError};
use tokio::io::{AsyncWrite, AsyncWriteExt};
use tokio::sync::mpsc;
use tracing_subscriber::fmt::MakeWriter;

use super::Stream;
use crate::{ActaError, Result};

#[derive(Debug, Default)]
struct Progress {
    sent: u64,
    written: u64,
}

#[derive(Debug)]
pub(crate) struct CustomAsyncWriter {
    sender: mpsc::Sender<Vec<u8>>,
    progress: Arc<(Mutex<Progress>, Condvar)>,
    dropped: AtomicU64,
}

#[derive(Debug)]
pub(crate) struct CustomAsyncGuard {
    progress: Arc<(Mutex<Progress>, Condvar)>,
}

impl CustomAsyncGuard {
    pub(crate) fn flush(&self) {
        let (mutex, condition) = &*self.progress;
        let mut progress = mutex.lock().unwrap_or_else(PoisonError::into_inner);
        let target = progress.sent;
        while progress.written < target {
            progress = condition
                .wait(progress)
                .unwrap_or_else(PoisonError::into_inner);
        }
        drop(progress);
    }
}

impl CustomAsyncWriter {
    pub(crate) fn new(stream: Stream, capacity: usize) -> Result<(Self, CustomAsyncGuard)> {
        tokio::runtime::Handle::try_current().map_err(|_| ActaError::RuntimeUnavailable)?;
        let progress = Arc::new((Mutex::new(Progress::default()), Condvar::new()));
        let (sender, mut receiver) = mpsc::channel::<Vec<u8>>(capacity);

        tokio::spawn({
            let progress = Arc::clone(&progress);
            async move {
                let mut writer: Box<dyn AsyncWrite + Unpin + Send> = match stream {
                    Stream::Out => Box::new(tokio::io::stdout()),
                    Stream::Err => Box::new(tokio::io::stderr()),
                };
                while let Some(data) = receiver.recv().await {
                    if let Err(e) = writer.write_all(&data).await {
                        let _unused = writeln!(io::stderr(), "async writer error: {e}");
                    }
                    let caught_up = {
                        let (mutex, _) = &*progress;
                        let mut state = mutex.lock().unwrap_or_else(PoisonError::into_inner);
                        state.written += 1;
                        state.written == state.sent
                    };
                    if caught_up {
                        let _unused = writer.flush().await;
                    }
                    progress.1.notify_all();
                }
            }
        });

        let guard = CustomAsyncGuard {
            progress: Arc::clone(&progress),
        };
        Ok((
            Self {
                sender,
                progress,
                dropped: AtomicU64::new(0),
            },
            guard,
        ))
    }
}

impl Write for &CustomAsyncWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self.sender.try_reserve() {
            Ok(permit) => {
                let (mutex, _) = &*self.progress;
                mutex.lock().unwrap_or_else(PoisonError::into_inner).sent += 1;
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
