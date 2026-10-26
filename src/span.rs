use crate::event::JsonFieldValue;
use serde_json::{Map, Value};
use std::{
    collections::HashMap,
    sync::{Mutex, PoisonError},
};
use tracing::{Dispatch, Metadata, Span as TracingSpan};

#[derive(Clone, Debug)]
pub(crate) struct Span(
    pub(crate) TracingSpan,
    pub(crate) &'static Metadata<'static>,
);

#[derive(Debug, Default)]
pub(crate) struct Spans(Mutex<SpanState>);

#[derive(Debug, Default)]
struct SpanState {
    next: u32,
    spans: HashMap<u32, Span>,
}

impl Spans {
    pub(crate) fn get(&self, id: u32) -> Result<Span, String> {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .spans
            .get(&id)
            .cloned()
            .ok_or_else(|| format!("invalid span: {id}"))
    }

    pub(crate) fn insert(&self, span: Span) -> Result<u32, String> {
        let mut state = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        let id = state
            .next
            .checked_add(1)
            .ok_or_else(|| String::from("span handles exhausted"))?;
        state.next = id;
        state.spans.insert(id, span);
        drop(state);
        Ok(id)
    }

    pub(crate) fn close(&self, id: u32) -> Result<(), String> {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .spans
            .remove(&id)
            .ok_or_else(|| format!("invalid span: {id}"))?;
        Ok(())
    }
}

impl Span {
    #[allow(clippy::single_call_fn)]
    pub(crate) fn new(
        dispatch: &Dispatch,
        metadata: &'static Metadata<'static>,
        event: &crate::Event,
        parent: Option<&Self>,
    ) -> Self {
        if !dispatch.enabled(metadata) {
            return Self(TracingSpan::new_disabled(metadata), metadata);
        }

        let mut recorded = Vec::new();

        if let Some(field) = metadata.fields().field("message") {
            recorded.push((field, JsonFieldValue::String(&event.message)));
        }

        for (name, value) in &event.fields {
            if let Some(field) = metadata.fields().field(name.as_str())
                && field.name() != "message"
            {
                recorded.push((field, JsonFieldValue::new(value)));
            }
        }

        let mut values = vec![None; metadata.fields().len()];
        for (field, value) in &recorded {
            if let Some(slot) = values.get_mut(field.index()) {
                *slot = Some(value.as_value());
            }
        }
        let values = metadata.fields().value_set_all(&values);
        let parent = parent.and_then(|span| span.0.id());

        Self(
            TracingSpan::child_of_with(parent, metadata, &values, dispatch),
            metadata,
        )
    }

    pub(crate) fn record(&self, fields: &Map<String, Value>) -> Result<(), String> {
        let field_set = self.1.fields();

        for name in fields.keys() {
            if field_set.field(name.as_str()).is_none() {
                return Err(format!("unknown span field: {name}"));
            }
        }

        let recorded = fields
            .iter()
            .filter_map(|(name, value)| {
                field_set
                    .field(name.as_str())
                    .map(|field| (field, JsonFieldValue::new(value)))
            })
            .collect::<Vec<_>>();
        let mut values = vec![None; field_set.len()];
        for (field, value) in &recorded {
            if let Some(slot) = values.get_mut(field.index()) {
                *slot = Some(value.as_value());
            }
        }
        let values = field_set.value_set_all(&values);
        self.0.record_all(&values);
        Ok(())
    }
}
