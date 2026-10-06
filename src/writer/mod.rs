use std::sync::Arc;
use tracing::Metadata;
use tracing_subscriber::fmt::MakeWriter;
use tracing_subscriber::fmt::writer::BoxMakeWriter;

#[derive(Debug)]
pub(crate) struct SharedWriter(pub(crate) Arc<BoxMakeWriter>);

impl<'a> MakeWriter<'a> for SharedWriter {
    type Writer = <BoxMakeWriter as MakeWriter<'a>>::Writer;

    fn make_writer(&'a self) -> Self::Writer {
        self.0.make_writer()
    }

    fn make_writer_for(&'a self, metadata: &Metadata<'_>) -> Self::Writer {
        self.0.make_writer_for(metadata)
    }
}

#[cfg(any(acta_wasm_console, acta_async))]
#[derive(Clone, Copy, Debug)]
pub(crate) enum Stream {
    Out,
    Err,
}

#[cfg(acta_wasm_console)]
pub(crate) mod wasm;

#[cfg(feature = "custom-async")]
pub(crate) mod custom_async;

#[cfg(feature = "native-async")]
pub(crate) mod native_async;
