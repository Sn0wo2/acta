#[cfg(any(feature = "custom-async", feature = "native-async"))]
use crate::config::AsyncMode;
use crate::config::{ColorDepth, Config, Filter, Format, Style, Writer, WriterTarget};
use crate::fmt::Formatter;
use std::io;
#[cfg(feature = "file")]
use std::path::PathBuf;
use std::sync::Arc;
use tracing_subscriber::Registry;
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::layer::Layered;
use tracing_subscriber::prelude::*;

#[cfg(any(feature = "file", feature = "custom-async", feature = "native-async"))]
use crate::writer;

pub(crate) type BoxedLayer = Box<dyn tracing_subscriber::Layer<Registry> + Send + Sync>;

/// Newtype wrapping Vec<BoxedLayer> to avoid orphan-rule reliance on
/// tracing-subscriber's blanket `impl Layer<S> for Vec<L>`.
pub(crate) struct Layers(pub(crate) Vec<BoxedLayer>);

impl tracing_subscriber::Layer<Registry> for Layers {
    fn on_layer(&mut self, subscriber: &mut Registry) {
        for layer in &mut self.0 {
            layer.on_layer(subscriber);
        }
    }

    fn on_new_span(&self, attrs: &tracing::span::Attributes<'_>, id: &tracing::span::Id, ctx: tracing_subscriber::layer::Context<'_, Registry>) {
        for layer in &self.0 {
            layer.on_new_span(attrs, id, ctx.clone());
        }
    }

    fn on_event(&self, event: &tracing::Event<'_>, ctx: tracing_subscriber::layer::Context<'_, Registry>) {
        for layer in &self.0 {
            layer.on_event(event, ctx.clone());
        }
    }

    fn on_enter(&self, id: &tracing::span::Id, ctx: tracing_subscriber::layer::Context<'_, Registry>) {
        for layer in &self.0 {
            layer.on_enter(id, ctx.clone());
        }
    }

    fn on_exit(&self, id: &tracing::span::Id, ctx: tracing_subscriber::layer::Context<'_, Registry>) {
        for layer in &self.0 {
            layer.on_exit(id, ctx.clone());
        }
    }

    fn on_close(&self, id: tracing::span::Id, ctx: tracing_subscriber::layer::Context<'_, Registry>) {
        for layer in &self.0 {
            layer.on_close(id.clone(), ctx.clone());
        }
    }

    fn on_record(&self, id: &tracing::span::Id, values: &tracing::span::Record<'_>, ctx: tracing_subscriber::layer::Context<'_, Registry>) {
        for layer in &self.0 {
            layer.on_record(id, values, ctx.clone());
        }
    }

    fn on_follows_from(&self, id: &tracing::span::Id, follows: &tracing::span::Id, ctx: tracing_subscriber::layer::Context<'_, Registry>) {
        for layer in &self.0 {
            layer.on_follows_from(id, follows, ctx.clone());
        }
    }

    fn on_id_change(&self, old: &tracing::span::Id, new: &tracing::span::Id, ctx: tracing_subscriber::layer::Context<'_, Registry>) {
        for layer in &self.0 {
            layer.on_id_change(old, new, ctx.clone());
        }
    }

    fn enabled(&self, metadata: &tracing::Metadata<'_>, ctx: tracing_subscriber::layer::Context<'_, Registry>) -> bool {
        self.0.iter().any(|layer| layer.enabled(metadata, ctx.clone()))
    }

    fn event_enabled(&self, event: &tracing::Event<'_>, ctx: tracing_subscriber::layer::Context<'_, Registry>) -> bool {
        self.0.iter().any(|layer| layer.event_enabled(event, ctx.clone()))
    }
}

pub(crate) type InnerSubscriber = Layered<Layers, Registry>;
pub(crate) type ReloadHandle =
    tracing_subscriber::reload::Handle<tracing_subscriber::EnvFilter, InnerSubscriber>;

#[allow(clippy::single_call_fn)]
fn detect_color_depth(target: &WriterTarget) -> ColorDepth {
    use supports_color::Stream;
    let stream = match *target {
        WriterTarget::Stdout => Stream::Stdout,
        WriterTarget::Stderr => Stream::Stderr,
        #[cfg(feature = "file")]
        WriterTarget::File { .. } => return ColorDepth::NoColor,
        #[cfg(any(feature = "custom-async", feature = "native-async"))]
        WriterTarget::AsyncStdout(_) => Stream::Stdout,
        #[cfg(any(feature = "custom-async", feature = "native-async"))]
        WriterTarget::AsyncStderr(_) => Stream::Stderr,
    };

    if let Some(level) = supports_color::on_cached(stream) {
        if level.has_16m {
            return ColorDepth::TrueColor;
        }
        if level.has_256 {
            return ColorDepth::Ansi256;
        }
        if level.has_basic {
            return ColorDepth::Ansi16;
        }
    }

    ColorDepth::NoColor
}

fn build_formatter(
    writer: &Writer,
    color_depth: ColorDepth,
    shared_handle: Option<Arc<arc_swap::ArcSwap<Style>>>,
) -> Formatter {
    let mut f = shared_handle
        .as_ref()
        .map_or_else(Formatter::new, |handle| {
            Formatter::new_with_handle(handle.clone())
        });
    f = f
        .with_style_config(writer.style)
        .with_show_path(writer.show_path)
        .with_show_spans(writer.show_spans)
        .with_color_depth(color_depth);
    if let Some(tf) = &writer.time_format {
        f = f.with_time_format(tf);
    }
    f
}

pub fn build_layer(writer: &Writer) -> BoxedLayer {
    build_layer_with(writer, None)
}

fn build_layer_with(
    writer: &Writer,
    shared_handle: Option<Arc<arc_swap::ArcSwap<Style>>>,
) -> BoxedLayer {
    let color_depth = writer.color_depth.unwrap_or_else(|| {
        if writer.ansi {
            detect_color_depth(&writer.target)
        } else {
            ColorDepth::NoColor
        }
    });
    let formatter = build_formatter(writer, color_depth, shared_handle);

    let base = || {
        tracing_subscriber::fmt::Layer::default()
            .with_thread_ids(false)
            .with_thread_names(false)
            .with_span_events(FmtSpan::NONE)
    };

    macro_rules! write_to {
        ($layer:expr $(,)?) => {{
            match &writer.target {
                WriterTarget::Stdout => $layer.with_writer(io::stdout).boxed(),
                WriterTarget::Stderr => $layer.with_writer(io::stderr).boxed(),
                #[cfg(feature = "custom-async")]
                WriterTarget::AsyncStdout(AsyncMode::Custom) => $layer
                    .with_writer(writer::async_writer_for(writer::AsyncWriterTarget::Stdout))
                    .boxed(),
                #[cfg(feature = "native-async")]
                WriterTarget::AsyncStdout(AsyncMode::Native) => $layer
                    .with_writer(writer::native_async_writer(
                        writer::AsyncWriterTarget::Stdout,
                    ))
                    .boxed(),
                #[cfg(feature = "custom-async")]
                WriterTarget::AsyncStderr(AsyncMode::Custom) => $layer
                    .with_writer(writer::async_writer_for(writer::AsyncWriterTarget::Stderr))
                    .boxed(),
                #[cfg(feature = "native-async")]
                WriterTarget::AsyncStderr(AsyncMode::Native) => $layer
                    .with_writer(writer::native_async_writer(
                        writer::AsyncWriterTarget::Stderr,
                    ))
                    .boxed(),
                #[cfg(feature = "file")]
                WriterTarget::File { .. } => $layer.with_writer(std::io::sink).boxed(),
            }
        }};
    }

    match &writer.format {
        Format::Pretty(cfg) => write_to!(
            base()
                .pretty()
                .with_target(cfg.target)
                .with_file(cfg.file)
                .with_line_number(cfg.line_number)
                .with_ansi(writer.ansi)
        ),
        Format::Compact(cfg) => write_to!(
            base()
                .with_target(cfg.target)
                .with_file(cfg.file)
                .with_line_number(cfg.line_number)
                .with_ansi(writer.ansi)
                .event_format(formatter)
        ),
        Format::Json(cfg) => write_to!(
            base()
                .json()
                .with_target(cfg.target)
                .with_file(cfg.file)
                .with_line_number(cfg.line_number)
                .with_current_span(cfg.current_span)
                .with_span_list(cfg.span_list)
                .flatten_event(cfg.flatten_event)
                .with_ansi(writer.ansi)
        ),
    }
}

#[cfg(feature = "file")]
#[allow(clippy::single_call_fn)]
fn build_file_layer(writer: &Writer, file_writer: writer::FileWriter) -> BoxedLayer {
    let base = || {
        tracing_subscriber::fmt::Layer::default()
            .with_thread_ids(false)
            .with_thread_names(false)
            .with_span_events(FmtSpan::NONE)
    };

    match &writer.format {
        Format::Pretty(cfg) => base()
            .pretty()
            .with_target(cfg.target)
            .with_file(cfg.file)
            .with_line_number(cfg.line_number)
            .with_ansi(false)
            .with_writer(file_writer)
            .boxed(),
        Format::Compact(cfg) => base()
            .with_target(cfg.target)
            .with_file(cfg.file)
            .with_line_number(cfg.line_number)
            .with_ansi(false)
            .event_format(build_formatter(writer, ColorDepth::NoColor, None))
            .with_writer(file_writer)
            .boxed(),
        Format::Json(cfg) => base()
            .json()
            .with_target(cfg.target)
            .with_file(cfg.file)
            .with_line_number(cfg.line_number)
            .with_current_span(cfg.current_span)
            .with_span_list(cfg.span_list)
            .flatten_event(cfg.flatten_event)
            .with_ansi(false)
            .with_writer(file_writer)
            .boxed(),
    }
}

pub fn init(config: impl Into<Config>) -> crate::Result<TracingGuard> {
    let Config { filter, writers } = config.into();
    let shared_style = Arc::new(arc_swap::ArcSwap::new(Arc::new(
        writers.first().map_or_else(Style::default, |w| w.style),
    )));

    let mut layers: Vec<BoxedLayer> = Vec::with_capacity(writers.len());
    #[cfg(feature = "file")]
    #[cfg(feature = "file")]
    let mut file_guard = None;
    #[cfg(feature = "file")]
    let mut log_path = None;

    for writer in writers {
        #[cfg(feature = "file")]
        let eligible = !matches!(writer.target, WriterTarget::File { .. });
        #[cfg(not(feature = "file"))]
        let eligible = true;

        let handle = eligible.then(|| shared_style.clone());

        #[cfg(feature = "file")]
        if let WriterTarget::File { ref path, rotation } = writer.target {
            let (file_w, guard, resolved_path) = writer::file::build_file_layer(path, rotation)?;
            file_guard = Some(guard);
            log_path = Some(resolved_path);
            layers.push(build_file_layer(&writer, file_w));
            continue;
        }

        layers.push(build_layer_with(&writer, handle));
    }

    let env_filter = tracing_subscriber::EnvFilter::try_new(filter.as_directive())?;
    let (env_filter_layer, raw) = tracing_subscriber::reload::Layer::new(env_filter);

    let subscriber = Registry::default().with(Layers(layers)).with(env_filter_layer);

    let _ = tracing_log::LogTracer::init();
    tracing::subscriber::set_global_default(subscriber)?;

    Ok(TracingGuard {
        raw,
        filter,
        style: shared_style,
        #[cfg(feature = "file")]
        worker_guard: file_guard,
        #[cfg(feature = "file")]
        log_path,
    })
}

#[must_use = "dropping TracingGuard will release associated resources"]
pub struct TracingGuard {
    pub(crate) raw: ReloadHandle,
    pub(crate) filter: Filter,
    pub(crate) style: Arc<arc_swap::ArcSwap<Style>>,
    #[cfg(feature = "file")]
    pub(crate) worker_guard: Option<writer::LogHandle>,
    #[cfg(feature = "file")]
    pub(crate) log_path: Option<PathBuf>,
}

impl std::fmt::Debug for TracingGuard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut d = f.debug_struct("TracingGuard");
        let _ = d.field("filter", &self.filter);
        #[cfg(feature = "file")]
        let _ = d.field("log_path", &self.log_path);
        #[cfg(feature = "file")]
        let _ = d.field("has_file_guard", &self.worker_guard.is_some());
        d.finish_non_exhaustive()
    }
}

impl TracingGuard {
    pub fn with_style(&mut self, f: impl FnOnce(&mut Style)) {
        let mut style = **self.style.load();
        f(&mut style);
        self.style.store(Arc::new(style));
    }

    pub fn set_filter(&mut self, filter: Filter) -> crate::Result<()> {
        let env_filter = tracing_subscriber::EnvFilter::try_new(filter.as_directive())?;
        self.raw.modify(|f| *f = env_filter)?;
        self.filter = filter;
        Ok(())
    }

    pub fn set_level(&mut self, level: crate::config::Level) -> crate::Result<()> {
        self.filter = Filter::new(level);
        self.apply_current_filter()
    }

    pub fn set_target_level(
        &mut self,
        target: impl Into<compact_str::CompactString>,
        level: crate::config::Level,
    ) -> crate::Result<()> {
        self.filter.with_target(target, level);
        self.apply_current_filter()
    }

    pub fn remove_target_level(&mut self, target: &str) -> crate::Result<()> {
        self.filter.remove_target(target);
        self.apply_current_filter()
    }

    fn apply_current_filter(&self) -> crate::Result<()> {
        let env_filter = tracing_subscriber::EnvFilter::try_new(self.filter.as_directive())?;
        self.raw.modify(|f| *f = env_filter)?;
        Ok(())
    }

    #[cfg(feature = "file")]
    pub fn log_path(&self) -> Option<&std::path::Path> {
        self.log_path.as_deref()
    }
}
