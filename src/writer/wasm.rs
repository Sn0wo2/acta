use std::{fmt::Write as _, io};

use anstyle_parse::Perform;
use smallvec::SmallVec;
use tracing_subscriber::fmt::MakeWriter;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::*;

use super::Stream;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console, js_name = log, variadic)]
    fn console_log(args: &[JsValue]);
    #[wasm_bindgen(js_namespace = console, js_name = error, variadic)]
    fn console_error(args: &[JsValue]);
    #[wasm_bindgen(js_namespace = ["globalThis"], js_name = hasOwnProperty)]
    fn global_has_own(key: &str) -> bool;
}

impl Stream {
    fn send(self, args: &[JsValue]) {
        match self {
            Self::Out => console_log(args),
            Self::Err => console_error(args),
        }
    }
}

const BOLD: u8 = 1;
const DIM: u8 = 2;
const ITALIC: u8 = 4;
const UNDERLINE: u8 = 8;
const STRIKE: u8 = 16;

#[derive(Default)]
struct ANSI2CSS {
    segments: Vec<(String, String)>,
    text: String,
    fg: Option<(u8, u8, u8)>,
    bg: Option<(u8, u8, u8)>,
    effects: u8,
}

impl ANSI2CSS {
    fn flush(&mut self) {
        if self.text.is_empty() {
            return;
        }
        let mut css = String::new();
        if let Some((r, g, b)) = self.fg {
            let _ = write!(css, "color:rgb({r},{g},{b});");
        }
        if let Some((r, g, b)) = self.bg {
            let _ = write!(css, "background-color:rgb({r},{g},{b});");
        }
        if self.effects & BOLD != 0 {
            css.push_str("font-weight:bold;");
        }
        if self.effects & ITALIC != 0 {
            css.push_str("font-style:italic;");
        }
        if self.effects & UNDERLINE != 0 {
            css.push_str("text-decoration:underline;");
        }
        if self.effects & STRIKE != 0 {
            css.push_str("text-decoration:line-through;");
        }
        if self.effects & DIM != 0 {
            css.push_str("opacity:0.7;");
        }
        self.segments.push((css, std::mem::take(&mut self.text)));
    }
}

impl Perform for ANSI2CSS {
    fn print(&mut self, c: char) {
        self.text.push(c);
    }

    fn execute(&mut self, byte: u8) {
        if byte == b'\n' {
            self.text.push('\n');
        }
    }

    fn csi_dispatch(
        &mut self,
        params: &anstyle_parse::Params,
        _intermediates: &[u8],
        _ignore: bool,
        action: u8,
    ) {
        if action != b'm' {
            return;
        }
        self.flush();
        let nums: SmallVec<[u16; 8]> = params.iter().flatten().copied().collect();
        let mut i = 0;
        while let Some(&n) = nums.get(i) {
            match n {
                0 => {
                    self.fg = None;
                    self.bg = None;
                    self.effects = 0;
                }
                1 => self.effects |= BOLD,
                2 => self.effects |= DIM,
                3 => self.effects |= ITALIC,
                4 => self.effects |= UNDERLINE,
                9 => self.effects |= STRIKE,
                22 => self.effects &= !(BOLD | DIM),
                23 => self.effects &= !ITALIC,
                24 => self.effects &= !UNDERLINE,
                29 => self.effects &= !STRIKE,
                39 => self.fg = None,
                49 => self.bg = None,
                30..=37 => self.fg = Some(ansi_colours::rgb_from_ansi256((n - 30) as u8)),
                40..=47 => self.bg = Some(ansi_colours::rgb_from_ansi256((n - 40) as u8)),
                90..=97 => self.fg = Some(ansi_colours::rgb_from_ansi256((n - 90 + 8) as u8)),
                100..=107 => self.bg = Some(ansi_colours::rgb_from_ansi256((n - 100 + 8) as u8)),
                38 | 48 => {
                    let slot = if n == 38 { &mut self.fg } else { &mut self.bg };
                    match nums.get(i + 1..) {
                        Some([2, r, g, b, ..]) => {
                            *slot = Some((*r as u8, *g as u8, *b as u8));
                            i += 4;
                        }
                        Some([5, v, ..]) => {
                            *slot = Some(ansi_colours::rgb_from_ansi256(*v as u8));
                            i += 2;
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
            i += 1;
        }
    }
}

pub(crate) struct WasmWriter {
    target: Stream,
    css: bool,
    buf: Vec<u8>,
}

impl WasmWriter {
    pub(crate) fn new(stream: Stream) -> Self {
        Self {
            target: stream,
            css: global_has_own("window"),
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
        if self.css {
            let mut conv = ANSI2CSS {
                text: String::with_capacity(msg.len()),
                ..Default::default()
            };
            let mut parser = anstyle_parse::Parser::<anstyle_parse::Utf8Parser>::new();
            for byte in msg.as_bytes() {
                parser.advance(&mut conv, *byte);
            }
            conv.flush();

            if conv.segments.iter().all(|(css, _)| css.is_empty()) {
                self.target.send(&[JsValue::from_str(
                    &conv
                        .segments
                        .into_iter()
                        .map(|(_, text)| text)
                        .collect::<String>(),
                )]);
            } else {
                let mut fmt = String::with_capacity(msg.len());
                for (_, text) in &conv.segments {
                    fmt.push_str("%c");
                    fmt.push_str(text);
                }
                let mut args = Vec::with_capacity(conv.segments.len() + 1);
                args.push(JsValue::from_str(&fmt));
                args.extend(
                    conv.segments
                        .into_iter()
                        .map(|(css, _)| JsValue::from_str(&css)),
                );
                self.target.send(&args);
            }
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
            css: self.css,
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
