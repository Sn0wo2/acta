use napi::bindgen_prelude::{BigInt, Buffer};
use napi_derive::napi;

const DEFAULT_CAPACITY: u32 = 64 * 1024;
const DEFAULT_DRAIN: u32 = 64 * 1024;

fn reason(error: acta_abi::Error) -> napi::Error {
    napi::Error::from_reason(error.to_string())
}

#[napi]
pub struct Logger {
    inner: Option<acta_abi::Logger>,
}

#[napi]
impl Logger {
    #[napi(constructor)]
    pub fn new(config: Option<serde_json::Value>, capacity: Option<u32>) -> napi::Result<Self> {
        let config = config.unwrap_or_else(|| serde_json::Value::Object(serde_json::Map::new()));
        acta_abi::Logger::from_config_value(config, capacity.unwrap_or(DEFAULT_CAPACITY) as usize)
            .map(|logger| Self {
                inner: Some(logger),
            })
            .map_err(reason)
    }

    #[napi]
    pub fn log(&self, level: u32, message: String) -> napi::Result<()> {
        self.with(|logger| logger.log(level, &message).map_err(reason))
    }

    #[napi]
    pub fn event(&self, event: serde_json::Value) -> napi::Result<()> {
        self.with(|logger| logger.event_value(event).map_err(reason))
    }

    #[napi]
    pub fn create_span(&self, event: serde_json::Value) -> napi::Result<u32> {
        self.with(|logger| logger.create_span_value(event).map_err(reason))
    }

    #[napi]
    pub fn record_span(&self, id: u32, fields: serde_json::Value) -> napi::Result<()> {
        self.with(|logger| logger.record_span_value(id, fields).map_err(reason))
    }

    #[napi]
    pub fn close_span(&self, id: u32) -> napi::Result<()> {
        self.with(|logger| logger.close_span(id).map_err(reason))
    }

    #[napi]
    pub fn drain(&self, max_bytes: Option<u32>) -> napi::Result<Buffer> {
        self.with(|logger| {
            Ok(Buffer::from(
                logger.drain(max_bytes.unwrap_or(DEFAULT_DRAIN) as usize),
            ))
        })
    }

    #[napi]
    pub fn dropped(&self) -> napi::Result<BigInt> {
        self.with(|logger| {
            Ok(BigInt {
                sign_bit: false,
                words: vec![logger.dropped()],
            })
        })
    }

    #[napi]
    pub fn set_filter(&self, filter: String) -> napi::Result<()> {
        self.with(|logger| logger.set_filter(&filter).map_err(reason))
    }

    #[napi]
    pub fn close(&mut self) {
        self.inner.take();
    }

    fn with<T>(
        &self,
        operation: impl FnOnce(&acta_abi::Logger) -> napi::Result<T>,
    ) -> napi::Result<T> {
        operation(
            self.inner
                .as_ref()
                .ok_or_else(|| napi::Error::from_reason("Logger is closed"))?,
        )
    }
}

#[napi]
pub fn abi_version() -> u32 {
    acta_abi::ABI_VERSION
}

#[napi]
pub fn version() -> String {
    acta_abi::VERSION.to_owned()
}
