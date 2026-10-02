use std::io;

use tracing_subscriber::fmt::MakeWriter;
use wasm_bindgen::JsValue;

use super::Stream;
use crate::utils::ansi_css::ansi_to_css;
use crate::utils::wasm::{ConsoleTarget, use_css};

pub(crate) struct WasmWriter {
    target: ConsoleTarget,
    buf: Vec<u8>,
}

impl WasmWriter {
    pub(crate) const fn new(stream: Stream) -> Self {
        Self {
            target: match stream {
                Stream::Out => ConsoleTarget::Log,
                Stream::Err => ConsoleTarget::Error,
            },
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
        let msg = String::from_utf8_lossy(&data).into_owned();
        if use_css() {
            let (fmt, styles) = ansi_to_css(&msg);
            let args = std::iter::once(JsValue::from_str(&fmt))
                .chain(styles.iter().map(|s| JsValue::from_str(s)))
                .collect::<Vec<_>>();
            self.target.send(&args);
        } else {
            self.target.send(&[JsValue::from_str(&msg)]);
        }
    }
}

impl MakeWriter<'_> for WasmWriter {
    type Writer = Self;

    fn make_writer(&self) -> Self {
        Self {
            target: self.target,
            buf: Vec::new(),
        }
    }
}

impl io::Write for WasmWriter {
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

impl Drop for WasmWriter {
    fn drop(&mut self) {
        self.emit();
    }
}
