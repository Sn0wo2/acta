use serde::de::DeserializeOwned;
use serde_json::Value;
use tracing::Level;

/// Bumped on any change to exported signatures or status semantics.
pub const ABI_VERSION: u32 = 2;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub const STATUS_OK: i32 = 0;
pub const STATUS_INVALID_ARGUMENT: i32 = 1;
pub const STATUS_INVALID_HANDLE: i32 = 2;
pub const STATUS_LOGGER_ERROR: i32 = 3;
pub const STATUS_INTERNAL_ERROR: i32 = 4;

#[derive(Debug)]
pub struct Error {
    pub status: i32,
    pub message: String,
}

impl Error {
    #[must_use]
    pub fn new(status: i32, message: impl Into<String>) -> Self {
        Self {
            status,
            message: message.into(),
        }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} (status {})", self.message, self.status)
    }
}

impl std::error::Error for Error {}

pub type Result<T> = std::result::Result<T, Error>;

fn invalid_argument(message: impl Into<String>) -> Error {
    Error::new(STATUS_INVALID_ARGUMENT, message)
}

fn logger_error(message: impl Into<String>) -> Error {
    Error::new(STATUS_LOGGER_ERROR, message)
}

#[derive(Debug)]
pub struct Logger {
    logger: acta::Logger,
    output: acta::BufferedWriter,
}

impl Logger {
    pub fn from_config_json(config_json: &str, capacity: usize) -> Result<Self> {
        let config: Value = serde_json::from_str(config_json)
            .map_err(|error| invalid_argument(format!("invalid config JSON: {error}")))?;
        Self::from_config_value(config, capacity)
    }

    pub fn from_config_value(config: Value, capacity: usize) -> Result<Self> {
        let output = acta::BufferedWriter::new(capacity);
        let config = acta::Config::from_json_with_writer(&config.to_string(), output.clone())
            .map_err(|error| logger_error(format!("invalid config: {error}")))?;
        let logger = acta::Logger::new(config).map_err(|error| logger_error(error.to_string()))?;
        Ok(Self { logger, output })
    }

    pub fn log(&self, level: u32, message: &str) -> Result<()> {
        let level = match level {
            0 => Level::ERROR,
            1 => Level::WARN,
            2 => Level::INFO,
            3 => Level::DEBUG,
            4 => Level::TRACE,
            other => return Err(invalid_argument(format!("invalid log level: {other}"))),
        };
        self.logger.log(level, message);
        Ok(())
    }

    pub fn event_json(&self, event_json: &str) -> Result<()> {
        self.event(&parse(event_json, "event")?)
    }

    pub fn event_value(&self, event: Value) -> Result<()> {
        self.event(&parse_value(event, "event")?)
    }

    pub fn event(&self, event: &acta::Event) -> Result<()> {
        self.logger
            .event(event)
            .map_err(|error| logger_error(error.to_string()))
    }

    pub fn create_span_json(&self, event_json: &str) -> Result<u32> {
        self.create_span(&parse(event_json, "span event")?)
    }

    pub fn create_span_value(&self, event: Value) -> Result<u32> {
        self.create_span(&parse_value(event, "span event")?)
    }

    pub fn create_span(&self, event: &acta::Event) -> Result<u32> {
        self.logger
            .create_span(event)
            .map_err(|error| logger_error(error.to_string()))
    }

    pub fn record_span_json(&self, id: u32, fields_json: &str) -> Result<()> {
        let fields: serde_json::Map<String, Value> = parse(fields_json, "span fields")?;
        self.record_span(id, &fields)
    }

    pub fn record_span_value(&self, id: u32, fields: Value) -> Result<()> {
        let fields = fields
            .as_object()
            .ok_or_else(|| invalid_argument("span fields must be an object"))?;
        self.record_span(id, fields)
    }

    pub fn record_span(&self, id: u32, fields: &serde_json::Map<String, Value>) -> Result<()> {
        self.logger
            .record_span(id, fields)
            .map_err(|error| logger_error(error.to_string()))
    }

    pub fn close_span(&self, id: u32) -> Result<()> {
        self.logger
            .close_span(id)
            .map_err(|error| logger_error(error.to_string()))
    }

    pub fn drain(&self, max_bytes: usize) -> Vec<u8> {
        self.output.drain(max_bytes)
    }

    pub fn dropped(&self) -> u64 {
        self.output.dropped()
    }

    pub fn set_filter(&self, filter: &str) -> Result<()> {
        let filter = filter
            .parse::<tracing_subscriber::EnvFilter>()
            .map_err(|error| invalid_argument(format!("invalid filter: {error}")))?;
        self.logger
            .set_filter(filter)
            .map_err(|error| logger_error(error.to_string()))
    }
}

fn parse<T: DeserializeOwned>(json: &str, what: &str) -> Result<T> {
    serde_json::from_str(json).map_err(|error| invalid_argument(format!("invalid {what}: {error}")))
}

fn parse_value<T: DeserializeOwned>(value: Value, what: &str) -> Result<T> {
    serde_json::from_value(value)
        .map_err(|error| invalid_argument(format!("invalid {what}: {error}")))
}
