#![deny(unsafe_code)]

#[cfg(target_arch = "wasm32")]
use acta::Logger as NativeLogger;
#[cfg(target_arch = "wasm32")]
use tracing::Level;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub struct Logger {
    inner: Option<NativeLogger>,
    output: acta::BufferedWriter,
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
impl Logger {
    #[wasm_bindgen(constructor)]
    pub fn new(config_json: &str, capacity: u32) -> Result<Logger, JsValue> {
        let output = acta::BufferedWriter::new(capacity as usize);
        let config = acta::Config::from_json_with_writer(config_json, output.clone())
            .map_err(|error| JsValue::from_str(&error.to_string()))?;
        let inner =
            NativeLogger::new(config).map_err(|error| JsValue::from_str(&error.to_string()))?;
        Ok(Self {
            inner: Some(inner),
            output,
        })
    }

    pub fn log(&self, level: u32, message: &str) -> Result<(), JsValue> {
        let level = match level {
            0 => Level::ERROR,
            1 => Level::WARN,
            2 => Level::INFO,
            3 => Level::DEBUG,
            4 => Level::TRACE,
            _ => return Err(JsValue::from_str(&format!("invalid log level: {level}"))),
        };
        self.logger()?.log(level, message);
        Ok(())
    }

    pub fn drain(&self, max_bytes: u32) -> Result<Vec<u8>, JsValue> {
        self.logger()?;
        Ok(self.output.drain(max_bytes as usize))
    }

    pub fn event(&self, event_json: &str) -> Result<(), JsValue> {
        let logger = self.logger()?;
        let event = serde_json::from_str(event_json)
            .map_err(|error| JsValue::from_str(&error.to_string()))?;
        logger
            .event(&event)
            .map_err(|error| JsValue::from_str(&error.to_string()))
    }

    #[wasm_bindgen(js_name = createSpan)]
    pub fn create_span(&self, event_json: &str) -> Result<u32, JsValue> {
        let logger = self.logger()?;
        let event = serde_json::from_str(event_json)
            .map_err(|error| JsValue::from_str(&error.to_string()))?;
        logger
            .create_span(&event)
            .map_err(|error| JsValue::from_str(&error.to_string()))
    }

    #[wasm_bindgen(js_name = recordSpan)]
    pub fn record_span(&self, id: u32, fields_json: &str) -> Result<(), JsValue> {
        let logger = self.logger()?;
        let fields =
            serde_json::from_str::<serde_json::Map<String, serde_json::Value>>(fields_json)
                .map_err(|error| JsValue::from_str(&error.to_string()))?;
        logger
            .record_span(id, &fields)
            .map_err(|error| JsValue::from_str(&error.to_string()))
    }

    #[wasm_bindgen(js_name = closeSpan)]
    pub fn close_span(&self, id: u32) -> Result<(), JsValue> {
        self.logger()?
            .close_span(id)
            .map_err(|error| JsValue::from_str(&error.to_string()))
    }

    pub fn dropped(&self) -> Result<u64, JsValue> {
        self.logger()?;
        Ok(self.output.dropped())
    }

    #[wasm_bindgen(js_name = setFilter)]
    pub fn set_filter(&self, filter: &str) -> Result<(), JsValue> {
        self.logger()?
            .set_filter(
                filter
                    .parse()
                    .map_err(|_| JsValue::from_str("invalid filter"))?,
            )
            .map_err(|error| JsValue::from_str(&error.to_string()))
    }

    pub fn close(&mut self) {
        self.inner.take();
    }
}

#[cfg(target_arch = "wasm32")]
impl Logger {
    fn logger(&self) -> Result<&NativeLogger, JsValue> {
        self.inner
            .as_ref()
            .ok_or_else(|| JsValue::from_str("Logger is closed"))
    }
}
