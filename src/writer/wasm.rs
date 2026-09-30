
use std::io;

#[derive(Clone, Copy, Debug)]
enum ConsoleTarget {
    Log,
    Error,
}

impl ConsoleTarget {
    fn dispatch(self, msg: &str) {
        match self {
            ConsoleTarget::Log => web_sys::console::log_1(&msg.into()),
            ConsoleTarget::Error => web_sys::console::error_1(&msg.into()),
        }
    }
}

#[derive(Debug)]
pub(crate) struct ConsoleSink {
    target: ConsoleTarget,
    buf: Vec<u8>,
}

impl ConsoleSink {
    const fn new(target: ConsoleTarget) -> Self {
        Self {
            target,
            buf: Vec::new(),
        }
    }

    fn emit(&mut self) {
        while matches!(self.buf.last(), Some(b'\n' | b'\r')) {
            self.buf.pop();
        }
        if self.buf.is_empty() {
            return;
        }
        let data = std::mem::take(&mut self.buf);
        let msg = String::from_utf8_lossy(&data);
        self.target.dispatch(&msg);
    }
}

impl io::Write for ConsoleSink {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.buf.extend_from_slice(buf);
        if buf.contains(&b'\n') {
            self.emit();
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.emit();
        Ok(())
    }
}

impl Drop for ConsoleSink {
    fn drop(&mut self) {
        self.emit();
    }
}

#[must_use]
pub(crate) const fn log_sink() -> ConsoleSink {
    ConsoleSink::new(ConsoleTarget::Log)
}

#[must_use]
pub(crate) const fn error_sink() -> ConsoleSink {
    ConsoleSink::new(ConsoleTarget::Error)
}
