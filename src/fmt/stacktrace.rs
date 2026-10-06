use std::backtrace::{Backtrace, BacktraceStatus};
use std::fmt::{self, Write as _};
use tracing::{Event, Level, Subscriber};
use tracing_subscriber::fmt::format::Writer;
use tracing_subscriber::fmt::{FmtContext, FormatEvent, FormatFields};
use tracing_subscriber::registry::LookupSpan;

#[derive(Debug, Clone)]
pub(crate) struct Stacktrace<F> {
    pub(crate) inner: F,
    pub(crate) levels: u8,
    pub(crate) flatten_event: Option<bool>,
}

impl<S, N, F> FormatEvent<S, N> for Stacktrace<F>
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
    F: FormatEvent<S, N>,
{
    fn format_event(
        &self,
        ctx: &FmtContext<'_, S, N>,
        mut writer: Writer<'_>,
        event: &Event<'_>,
    ) -> fmt::Result {
        if self.levels == 0
            || self.levels
                & match *event.metadata().level() {
                    Level::ERROR => 1,
                    Level::WARN => 2,
                    Level::INFO => 4,
                    Level::DEBUG => 8,
                    Level::TRACE => 16,
                }
                == 0
        {
            return self.inner.format_event(ctx, writer, event);
        }

        let backtrace = Backtrace::force_capture();
        if backtrace.status() != BacktraceStatus::Captured {
            return self.inner.format_event(ctx, writer, event);
        }

        let Some(flatten_event) = self.flatten_event else {
            self.inner.format_event(ctx, writer.by_ref(), event)?;
            return writeln!(writer, "Stacktrace:\n{backtrace}");
        };

        let mut buffer = String::new();
        self.inner
            .format_event(ctx, Writer::new(&mut buffer), event)?;
        let Ok(mut json) = serde_json::from_str::<serde_json::Value>(&buffer) else {
            return writer.write_str(&buffer);
        };
        let Some(fields) = (if flatten_event {
            json.as_object_mut()
        } else {
            json.get_mut("fields")
                .and_then(serde_json::Value::as_object_mut)
        }) else {
            return writer.write_str(&buffer);
        };
        let key = if !fields.contains_key("message") && fields.contains_key("msg") {
            "msg"
        } else {
            "message"
        };
        let message = fields.entry(key).or_insert(serde_json::Value::Null);
        let mut text = match message.take() {
            serde_json::Value::String(text) => text,
            serde_json::Value::Null => String::new(),
            value => value.to_string(),
        };
        if !text.is_empty() {
            text.push('\n');
        }
        write!(text, "Stacktrace:\n{backtrace}")?;
        *message = serde_json::Value::String(text);
        writeln!(writer, "{json}")
    }
}
