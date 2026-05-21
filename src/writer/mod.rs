#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub enum AsyncWriterTarget {
    Stdout,
    Stderr,
}

#[cfg(feature = "custom-async")]
pub mod custom;

#[cfg(feature = "custom-async")]
#[allow(clippy::module_name_repetitions)]
pub use custom::{AsyncWriter, async_writer_for};

#[cfg(feature = "native-async")]
pub mod native;

#[cfg(feature = "native-async")]
#[allow(clippy::module_name_repetitions)]
pub use native::{NativeAsyncWriter, native_async_writer};

#[cfg(feature = "file")]
pub mod file;

#[cfg(feature = "file")]
pub(crate) use file::FileWriter;
#[cfg(feature = "file")]
pub use file::{LogHandle, resolve_log_path, rotate_log_file};

#[cfg(test)]
mod test;
