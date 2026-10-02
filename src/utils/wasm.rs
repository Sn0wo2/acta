use std::sync::OnceLock;

use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console, js_name = log, variadic)]
    pub(crate) fn console_log(args: &[JsValue]);
    #[wasm_bindgen(js_namespace = console, js_name = error, variadic)]
    pub(crate) fn console_error(args: &[JsValue]);
    #[wasm_bindgen(js_namespace = ["globalThis"], js_name = hasOwnProperty)]
    fn global_has_own(key: &str) -> bool;
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum ConsoleTarget {
    Log,
    Error,
}

impl ConsoleTarget {
    pub(crate) fn send(self, args: &[JsValue]) {
        match self {
            Self::Log => console_log(args),
            Self::Error => console_error(args),
        }
    }
}

#[allow(clippy::single_call_fn)]
pub(crate) fn use_css() -> bool {
    static CSS: OnceLock<bool> = OnceLock::new();
    *CSS.get_or_init(|| global_has_own("window"))
}
