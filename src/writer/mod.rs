use std::io;

use tracing_subscriber::fmt::writer::BoxMakeWriter;

#[cfg(acta_async)]
use crate::config::AsyncMode;
use crate::config::WriterTarget;
#[cfg(feature = "file")]
use std::path::PathBuf;
#[cfg(feature = "file")]
use tracing_appender::non_blocking::WorkerGuard;

#[cfg(any(acta_wasm_console, acta_async))]
#[derive(Clone, Copy, Debug)]
pub(crate) enum Stream {
    Out,
    Err,
}

pub(crate) struct BuiltWriter {
    pub(crate) make_writer: BoxMakeWriter,
    #[cfg(feature = "file")]
    pub(crate) file: Option<(WorkerGuard, PathBuf)>,
}

impl BuiltWriter {
    const fn simple(make_writer: BoxMakeWriter) -> Self {
        Self {
            make_writer,
            #[cfg(feature = "file")]
            file: None,
        }
    }
}

#[cfg_attr(not(feature = "file"), allow(clippy::unnecessary_wraps))]
pub(crate) fn build_writer(target: &WriterTarget) -> crate::Result<BuiltWriter> {
    match *target {
        #[cfg(not(acta_wasm_console))]
        WriterTarget::Stdout => Ok(BuiltWriter::simple(BoxMakeWriter::new(io::stdout))),

        #[cfg(not(acta_wasm_console))]
        WriterTarget::Stderr => Ok(BuiltWriter::simple(BoxMakeWriter::new(io::stderr))),

        #[cfg(acta_wasm_console)]
        WriterTarget::Stdout => Ok(BuiltWriter::simple(BoxMakeWriter::new(
            wasm::WasmWriter::new(Stream::Out),
        ))),

        #[cfg(acta_wasm_console)]
        WriterTarget::Stderr => Ok(BuiltWriter::simple(BoxMakeWriter::new(
            wasm::WasmWriter::new(Stream::Err),
        ))),

        #[cfg(feature = "file")]
        WriterTarget::File(ref config) => {
            let (writer, guard, path) = file::new(&config.path, config.rotation)?;
            Ok(BuiltWriter {
                make_writer: BoxMakeWriter::new(writer),
                file: Some((guard, path)),
            })
        }

        #[cfg(feature = "custom-async")]
        WriterTarget::AsyncStdout(AsyncMode::Custom { buffer_size }) => {
            Ok(BuiltWriter::simple(BoxMakeWriter::new(
                custom_async::CustomAsyncWriter::new(Stream::Out, buffer_size),
            )))
        }

        #[cfg(feature = "custom-async")]
        WriterTarget::AsyncStderr(AsyncMode::Custom { buffer_size }) => {
            Ok(BuiltWriter::simple(BoxMakeWriter::new(
                custom_async::CustomAsyncWriter::new(Stream::Err, buffer_size),
            )))
        }

        #[cfg(feature = "native-async")]
        WriterTarget::AsyncStdout(AsyncMode::Native) => Ok(BuiltWriter::simple(
            BoxMakeWriter::new(native_async::NativeAsyncWriter::new(Stream::Out)),
        )),

        #[cfg(feature = "native-async")]
        WriterTarget::AsyncStderr(AsyncMode::Native) => Ok(BuiltWriter::simple(
            BoxMakeWriter::new(native_async::NativeAsyncWriter::new(Stream::Err)),
        )),
    }
}

#[allow(clippy::single_call_fn)]
pub(crate) fn make_writer(target: &WriterTarget) -> BoxMakeWriter {
    #[cfg(feature = "file")]
    if matches!(target, WriterTarget::File(_)) {
        return BoxMakeWriter::new(io::sink);
    }
    match build_writer(target) {
        Ok(built) => built.make_writer,
        Err(_) => BoxMakeWriter::new(io::sink),
    }
}

#[cfg(acta_wasm_console)]
mod wasm;

#[cfg(feature = "custom-async")]
mod custom_async;

#[cfg(feature = "native-async")]
mod native_async;

#[cfg(feature = "file")]
mod file;
