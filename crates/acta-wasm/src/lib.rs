#![deny(unsafe_code)]

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub struct Logger {
    inner: Option<acta_abi::Logger>,
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
impl Logger {
    #[wasm_bindgen(constructor)]
    pub fn new(config_json: &str, capacity: u32) -> Result<Logger, JsValue> {
        acta_abi::Logger::from_config_json(config_json, capacity as usize)
            .map(|logger| Self {
                inner: Some(logger),
            })
            .map_err(js_error)
    }

    pub fn log(&self, level: u32, message: &str) -> Result<(), JsValue> {
        self.with(|logger| logger.log(level, message))
    }

    pub fn drain(&self, max_bytes: u32) -> Result<Vec<u8>, JsValue> {
        self.with(|logger| Ok(logger.drain(max_bytes as usize)))
    }

    pub fn event(&self, event_json: &str) -> Result<(), JsValue> {
        self.with(|logger| logger.event_json(event_json))
    }

    #[wasm_bindgen(js_name = createSpan)]
    pub fn create_span(&self, event_json: &str) -> Result<u32, JsValue> {
        self.with(|logger| logger.create_span_json(event_json))
    }

    #[wasm_bindgen(js_name = recordSpan)]
    pub fn record_span(&self, id: u32, fields_json: &str) -> Result<(), JsValue> {
        self.with(|logger| logger.record_span_json(id, fields_json))
    }

    #[wasm_bindgen(js_name = closeSpan)]
    pub fn close_span(&self, id: u32) -> Result<(), JsValue> {
        self.with(|logger| logger.close_span(id))
    }

    pub fn dropped(&self) -> Result<u64, JsValue> {
        self.with(|logger| Ok(logger.dropped()))
    }

    #[wasm_bindgen(js_name = setFilter)]
    pub fn set_filter(&self, filter: &str) -> Result<(), JsValue> {
        self.with(|logger| logger.set_filter(filter))
    }

    pub fn close(&mut self) {
        self.inner.take();
    }
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(js_name = abiVersion)]
pub fn abi_version() -> u32 {
    acta_abi::ABI_VERSION
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn version() -> String {
    acta_abi::VERSION.to_owned()
}

#[cfg(target_arch = "wasm32")]
impl Logger {
    fn with<T>(
        &self,
        operation: impl FnOnce(&acta_abi::Logger) -> acta_abi::Result<T>,
    ) -> Result<T, JsValue> {
        let logger = self
            .inner
            .as_ref()
            .ok_or_else(|| JsValue::from_str("Logger is closed"))?;
        operation(logger).map_err(js_error)
    }
}

#[cfg(target_arch = "wasm32")]
fn js_error(error: acta_abi::Error) -> JsValue {
    JsValue::from_str(&error.to_string())
}
