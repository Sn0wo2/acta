use std::fmt;
use std::mem::size_of;
use std::sync::{Mutex, OnceLock};

#[cfg(feature = "serde")]
use serde::{Deserialize, Deserializer, Serializer};
use tracing::callsite::Callsite;

const MAX_SCHEMA_BYTES: usize = 4 * 1024 * 1024;

#[derive(Clone, Debug)]
#[allow(clippy::exhaustive_structs)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct Event {
    #[cfg_attr(feature = "serde", serde(with = "level"))]
    pub level: tracing::Level,
    pub message: String,
    pub target: Option<String>,
    pub name: Option<String>,
    pub module_path: Option<String>,
    pub file: Option<String>,
    pub line: Option<u32>,
    pub parent: Option<u32>,
    pub fields: serde_json::Map<String, serde_json::Value>,
}

impl Event {
    pub fn new(level: tracing::Level, message: impl Into<String>) -> Self {
        Self {
            level,
            message: message.into(),
            target: None,
            name: None,
            module_path: None,
            file: None,
            line: None,
            parent: None,
            fields: serde_json::Map::new(),
        }
    }
}

impl Default for Event {
    fn default() -> Self {
        Self::new(tracing::Level::INFO, String::new())
    }
}

struct DynamicCallsite {
    metadata: OnceLock<tracing::Metadata<'static>>,
    registered: OnceLock<()>,
}

impl DynamicCallsite {
    fn register(&'static self) {
        self.registered
            .get_or_init(|| tracing::callsite::register(self));
    }
}

impl Callsite for DynamicCallsite {
    fn set_interest(&self, _interest: tracing::subscriber::Interest) {}

    fn metadata(&self) -> &tracing::Metadata<'_> {
        self.metadata.get().unwrap_or_else(|| std::process::abort())
    }
}

#[derive(Default)]
struct SchemaRegistry {
    schemas: Vec<&'static DynamicCallsite>,
    bytes: usize,
}

static SCHEMAS: OnceLock<Mutex<SchemaRegistry>> = OnceLock::new();

#[allow(clippy::single_call_fn)]
pub(crate) fn schema_for(
    event: &Event,
    field_names: &[&str],
    kind: tracing::metadata::Kind,
) -> crate::Result<&'static tracing::Metadata<'static>> {
    let target = event.target.as_deref().unwrap_or("acta");
    let name = event.name.as_deref().unwrap_or("event");
    let module_path = event.module_path.as_deref();
    let file = event.file.as_deref();

    let mut registry = SCHEMAS
        .get_or_init(|| Mutex::new(SchemaRegistry::default()))
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if let Some(callsite) = registry.schemas.iter().copied().find(|callsite| {
        let metadata = callsite.metadata();
        metadata.is_event() == kind.is_event()
            && metadata.is_span() == kind.is_span()
            && metadata.level() == &event.level
            && metadata.target() == target
            && metadata.name() == name
            && metadata.module_path() == module_path
            && metadata.file() == file
            && metadata.line() == event.line
            && metadata.fields().len() == field_names.len()
            && metadata
                .fields()
                .iter()
                .zip(field_names)
                .all(|(field, expected)| field.name() == *expected)
    }) {
        drop(registry);
        callsite.register();
        return Ok(callsite.metadata());
    }

    if registry.schemas.len() >= 4096 {
        return Err(crate::ActaError::EventSchemaLimit);
    }
    let cost = [
        (target != "acta").then_some(target),
        (name != "event").then_some(name),
        module_path,
        file,
    ]
    .into_iter()
    .flatten()
    .chain(
        field_names
            .iter()
            .copied()
            .filter(|field| *field != "message"),
    )
    .try_fold(0_usize, |bytes, value| {
        bytes.checked_add(value.len())?.checked_add(64)
    })
    .and_then(|bytes| {
        bytes
            .checked_add(field_names.len().checked_mul(size_of::<&'static str>())?)?
            .checked_add(size_of::<DynamicCallsite>())?
            .checked_add(256)
    })
    .filter(|cost| *cost <= MAX_SCHEMA_BYTES)
    .ok_or(crate::ActaError::EventSchemaBudget)?;
    if registry
        .bytes
        .checked_add(cost)
        .is_none_or(|bytes| bytes > MAX_SCHEMA_BYTES)
    {
        return Err(crate::ActaError::EventSchemaBudget);
    }
    registry
        .schemas
        .try_reserve(1)
        .map_err(|_| crate::ActaError::EventSchemaAllocation)?;
    registry.bytes += cost;

    let target = if target == "acta" {
        "acta"
    } else {
        Box::leak(target.to_owned().into_boxed_str())
    };
    let name = if name == "event" {
        "event"
    } else {
        Box::leak(name.to_owned().into_boxed_str())
    };
    let module_path = module_path.map(|value| {
        if value.is_empty() {
            ""
        } else {
            Box::leak(value.to_owned().into_boxed_str())
        }
    });
    let file = file.map(|value| {
        if value.is_empty() {
            ""
        } else {
            Box::leak(value.to_owned().into_boxed_str())
        }
    });
    let fields: Vec<&'static str> = field_names
        .iter()
        .map(|field| {
            if *field == "message" {
                "message"
            } else {
                Box::leak((*field).to_owned().into_boxed_str())
            }
        })
        .collect();
    let fields = Box::leak(fields.into_boxed_slice());
    let callsite = Box::leak(Box::new(DynamicCallsite {
        metadata: OnceLock::new(),
        registered: OnceLock::new(),
    }));
    let metadata = tracing::Metadata::new(
        name,
        target,
        event.level,
        file,
        event.line,
        module_path,
        tracing::field::FieldSet::new(fields, tracing::callsite::Identifier(callsite)),
        kind,
    );
    callsite.metadata.get_or_init(|| metadata);
    registry.schemas.push(callsite);
    drop(registry);
    callsite.register();
    Ok(callsite.metadata())
}

pub(crate) fn field_names(event: &Event) -> crate::Result<Vec<&str>> {
    if event.fields.contains_key("message") || event.fields.contains_key("msg") {
        let field = if event.fields.contains_key("message") {
            "message"
        } else {
            "msg"
        };
        return Err(crate::ActaError::EventReservedField(field.to_owned()));
    }

    let mut field_names: Vec<&str> = event.fields.keys().map(String::as_str).collect();
    field_names.push("message");
    field_names.sort_unstable();
    Ok(field_names)
}

#[allow(clippy::single_call_fn)]
pub(crate) fn emit(
    dispatch: &tracing::Dispatch,
    event: &Event,
    parent: Option<&crate::span::Span>,
) -> crate::Result<()> {
    let field_names = field_names(event)?;

    let metadata = schema_for(event, &field_names, tracing::metadata::Kind::EVENT)?;
    tracing::dispatcher::with_default(dispatch, || {
        if !dispatch.enabled(metadata) {
            return;
        }

        let mut values = Vec::with_capacity(event.fields.len());
        for field in metadata.fields().iter() {
            if field.name() != "message"
                && let Some(value) = event.fields.get(field.name())
            {
                values.push(JsonFieldValue::new(value));
            }
        }

        let message_value: &dyn tracing::field::Value = &event.message;
        let mut values = values.iter();
        let value_set: Vec<Option<&(dyn tracing::field::Value + '_)>> = metadata
            .fields()
            .iter()
            .map(|field| match field.name() {
                "message" => Some(message_value),
                _ => values.next().map(JsonFieldValue::as_value),
            })
            .collect();
        let value_set = metadata.fields().value_set_all(&value_set);
        let event = parent.map_or_else(
            || tracing::Event::new(metadata, &value_set),
            |parent| tracing::Event::new_child_of(parent.0.id(), metadata, &value_set),
        );
        dispatch.event(&event);
    });
    Ok(())
}

pub(crate) enum JsonFieldValue<'a> {
    Bool(bool),
    I64(i64),
    U64(u64),
    F64(f64),
    String(&'a str),
    Debug(tracing::field::DebugValue<JsonDebug<'a>>),
    DebugNumber(tracing::field::DebugValue<&'a serde_json::Number>),
}

impl<'a> JsonFieldValue<'a> {
    pub(crate) fn new(value: &'a serde_json::Value) -> Self {
        match value {
            serde_json::Value::Null
            | serde_json::Value::Array(_)
            | serde_json::Value::Object(_) => Self::Debug(tracing::field::debug(JsonDebug(value))),
            serde_json::Value::Bool(value) => Self::Bool(*value),
            serde_json::Value::Number(value) => {
                match (value.as_i64(), value.as_u64(), value.as_f64()) {
                    (Some(value), _, _) => Self::I64(value),
                    (_, Some(value), _) => Self::U64(value),
                    (_, _, Some(value)) => Self::F64(value),
                    (_, _, _) => Self::DebugNumber(tracing::field::debug(value)),
                }
            }
            serde_json::Value::String(value) => Self::String(value),
        }
    }

    #[allow(clippy::single_call_fn)]
    pub(crate) fn as_value(&self) -> &(dyn tracing::field::Value + '_) {
        match self {
            Self::Bool(value) => value,
            Self::I64(value) => value,
            Self::U64(value) => value,
            Self::F64(value) => value,
            Self::String(value) => value,
            Self::Debug(value) => value,
            Self::DebugNumber(value) => value,
        }
    }
}

pub(crate) struct JsonDebug<'a>(&'a serde_json::Value);

impl fmt::Debug for JsonDebug<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self.0, f)
    }
}

#[cfg(feature = "serde")]
#[allow(clippy::single_call_fn)]
mod level {
    use super::*;

    pub(super) fn serialize<S>(level: &tracing::Level, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(level.as_str())
    }

    pub(super) fn deserialize<'de, D>(deserializer: D) -> Result<tracing::Level, D::Error>
    where
        D: Deserializer<'de>,
    {
        let level = String::deserialize(deserializer)?;
        level.parse().map_err(serde::de::Error::custom)
    }
}
