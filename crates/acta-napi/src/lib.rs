use napi::Error;
use napi::bindgen_prelude::{BigInt, Buffer};
use napi_derive::napi;
use tracing::Level;

#[napi]
pub struct Logger {
    inner: acta::Logger,
    output: acta::BufferedWriter,
}

#[napi]
impl Logger {
    #[napi(constructor)]
    pub fn new(config: Option<serde_json::Value>, capacity: Option<u32>) -> napi::Result<Self> {
        let output = acta::BufferedWriter::new(capacity.unwrap_or(64 * 1024) as usize);
        let config = config
            .unwrap_or_else(|| serde_json::Value::Object(serde_json::Map::new()))
            .to_string();
        let config = acta::Config::from_json_with_writer(&config, output.clone())
            .map_err(|error| Error::from_reason(error.to_string()))?;
        let inner =
            acta::Logger::new(config).map_err(|error| Error::from_reason(error.to_string()))?;
        Ok(Self { inner, output })
    }

    #[napi]
    pub fn log(&self, level: u32, message: String) -> napi::Result<()> {
        let level = match level {
            0 => Level::ERROR,
            1 => Level::WARN,
            2 => Level::INFO,
            3 => Level::DEBUG,
            4 => Level::TRACE,
            _ => return Err(Error::from_reason(format!("invalid log level: {level}"))),
        };
        self.inner.log(level, &message);
        Ok(())
    }

    #[napi]
    pub fn drain(&self, max_bytes: u32) -> Buffer {
        Buffer::from(self.output.drain(max_bytes as usize))
    }

    #[napi]
    pub fn event(&self, event: serde_json::Value) -> napi::Result<()> {
        let event = serde_json::from_value::<acta::Event>(event)
            .map_err(|error| Error::from_reason(error.to_string()))?;
        self.inner
            .event(&event)
            .map_err(|error| Error::from_reason(error.to_string()))
    }

    #[napi]
    pub fn create_span(&self, event: serde_json::Value) -> napi::Result<u32> {
        let event = serde_json::from_value::<acta::Event>(event)
            .map_err(|error| Error::from_reason(error.to_string()))?;
        self.inner
            .create_span(&event)
            .map_err(|error| Error::from_reason(error.to_string()))
    }

    #[napi]
    pub fn record_span(&self, id: u32, fields: serde_json::Value) -> napi::Result<()> {
        let fields = fields
            .as_object()
            .ok_or_else(|| Error::from_reason("fields must be an object"))?;
        self.inner
            .record_span(id, fields)
            .map_err(|error| Error::from_reason(error.to_string()))
    }

    #[napi]
    pub fn close_span(&self, id: u32) -> napi::Result<()> {
        self.inner
            .close_span(id)
            .map_err(|error| Error::from_reason(error.to_string()))
    }

    #[napi]
    pub fn dropped(&self) -> BigInt {
        BigInt {
            sign_bit: false,
            words: vec![self.output.dropped()],
        }
    }

    #[napi]
    pub fn set_filter(&self, filter: String) -> napi::Result<()> {
        self.inner
            .set_filter(
                filter
                    .parse()
                    .map_err(|_| Error::from_reason("invalid filter"))?,
            )
            .map_err(|error| Error::from_reason(error.to_string()))
    }
}
