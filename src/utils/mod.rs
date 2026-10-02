#[cfg(acta_wasm_console)]
pub(crate) mod ansi_css;

#[cfg(not(target_arch = "wasm32"))]
pub(crate) mod terminal;

#[cfg(acta_wasm_console)]
pub(crate) mod wasm;

pub(crate) mod color;

pub(crate) mod time;
