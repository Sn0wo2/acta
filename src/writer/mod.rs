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

#[cfg(feature = "file")]
pub(crate) mod file;
