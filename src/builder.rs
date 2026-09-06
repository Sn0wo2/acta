#[cfg(any(feature = "custom-async", feature = "native-async"))]
use crate::config::AsyncMode;
use crate::config::{ColorDepth, Config, Filter, Format, Writer, WriterTarget};
use crate::fmt::Formatter;
use std::io;
#[cfg(feature = "file")]
use std::path::PathBuf;
use tracing_subscriber::Registry;
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::fmt::writer::BoxMakeWriter;
use tracing_subscriber::layer::Layered;
use tracing_subscriber::prelude::*;

#[cfg(any(feature = "file", feature = "custom-async", feature = "native-async"))]
use crate::writer;

pub(crate) type BoxedLayer = Box<dyn tracing_subscriber::Layer<Registry> + Send + Sync>;
pub(crate) type InnerSubscriber = Layered<Vec<BoxedLayer>, Registry>;
pub(crate) type ReloadHandle =
    tracing_subscriber::reload::Handle<tracing_subscriber::EnvFilter, InnerSubscriber>;

#[allow(clippy::single_call_fn)]
fn detect_color_depth(target: &WriterTarget) -> ColorDepth {
    use supports_color::Stream;
    let stream = match *target {
        WriterTarget::Stdout => Stream::Stdout,
        WriterTarget::Stderr => Stream::Stderr,
        #[cfg(feature = "file")]
        WriterTarget::File(_) => return ColorDepth::NoColor,
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

fn build_fmt_layer(
    writer: &Writer,
    make_writer: BoxMakeWriter,
    ansi: bool,
    color_depth: ColorDepth,
) -> BoxedLayer {
    let base = tracing_subscriber::fmt::Layer::default()
        .with_thread_ids(false)
        .with_thread_names(false)
        .with_span_events(FmtSpan::NONE)
        .with_writer(make_writer)
        .with_ansi(ansi);

    match &writer.format {
        Format::Pretty(cfg) => base
            .pretty()
            .with_target(cfg.target)
            .with_file(cfg.file)
            .with_line_number(cfg.line_number)
            .boxed(),
        Format::Compact(cfg) => {
            let mut formatter = Formatter::new()
                .with_style_config(writer.style)
                .with_show_path(writer.show_path)
                .with_show_spans(writer.show_spans)
                .with_color_depth(color_depth);
            if let Some(tf) = &writer.time_format {
                formatter = formatter.with_time_format(tf.clone());
            }
            base.with_target(cfg.target)
                .with_file(cfg.file)
                .with_line_number(cfg.line_number)
                .event_format(formatter)
                .boxed()
        }
        Format::Json(cfg) => base
            .json()
            .with_target(cfg.target)
            .with_file(cfg.file)
            .with_line_number(cfg.line_number)
            .with_current_span(cfg.current_span)
            .with_span_list(cfg.span_list)
            .flatten_event(cfg.flatten_event)
            .boxed(),
    }
}

/// Build a tracing layer from a [`Writer`] configuration.
///
/// Useful for ad-hoc subscriber setups such as demos or tests.
/// For production use, prefer [`init`] which handles file layers and
/// reload guards automatically.
pub fn build_layer(writer: &Writer) -> BoxedLayer {
    let color_depth = writer.color_depth.unwrap_or_else(|| {
        if writer.ansi {
            detect_color_depth(&writer.target)
        } else {
            ColorDepth::NoColor
        }
    });

    let make_writer = match &writer.target {
        WriterTarget::Stdout => BoxMakeWriter::new(io::stdout),
        WriterTarget::Stderr => BoxMakeWriter::new(io::stderr),
        #[cfg(feature = "custom-async")]
        WriterTarget::AsyncStdout(AsyncMode::Custom { buffer_size }) => BoxMakeWriter::new(
            writer::async_writer_for(writer::AsyncWriterTarget::Stdout, *buffer_size),
        ),
        #[cfg(feature = "native-async")]
        WriterTarget::AsyncStdout(AsyncMode::Native) => BoxMakeWriter::new(
            writer::native_async_writer(writer::AsyncWriterTarget::Stdout),
        ),
        #[cfg(feature = "custom-async")]
        WriterTarget::AsyncStderr(AsyncMode::Custom { buffer_size }) => BoxMakeWriter::new(
            writer::async_writer_for(writer::AsyncWriterTarget::Stderr, *buffer_size),
        ),
        #[cfg(feature = "native-async")]
        WriterTarget::AsyncStderr(AsyncMode::Native) => BoxMakeWriter::new(
            writer::native_async_writer(writer::AsyncWriterTarget::Stderr),
        ),
        #[cfg(feature = "file")]
        WriterTarget::File(_) => BoxMakeWriter::new(io::sink),
    };

    build_fmt_layer(writer, make_writer, writer.ansi, color_depth)
}

/// Initialize the global tracing subscriber.
///
/// Accepts anything convertible into a [`Config`]: a [`Level`](crate::Level),
/// a [`Filter`], a single [`Writer`], a `Vec<Writer>`, a
/// [`ConfigBuilder`](crate::ConfigBuilder), or a full [`Config`].
///
/// ```no_run
/// let _guard = acta::init(acta::Level::Debug)?;
/// # Ok::<(), acta::ActaError>(())
/// ```
pub fn init(config: impl Into<Config>) -> crate::Result<TracingGuard> {
    let Config { filter, writers } = config.into();
    let mut layers: Vec<BoxedLayer> = Vec::with_capacity(writers.len());

    #[cfg(feature = "file")]
    let mut file_guards = Vec::new();
    #[cfg(feature = "file")]
    let mut log_paths = Vec::new();

    for writer in writers {
        #[cfg(feature = "file")]
        if let WriterTarget::File(ref file_config) = writer.target {
            let (file_writer, guard, resolved_path) =
                writer::file::build_file_layer(&file_config.path, file_config.rotation)?;
            file_guards.push(guard);
            log_paths.push(resolved_path);
            layers.push(build_fmt_layer(
                &writer,
                BoxMakeWriter::new(file_writer),
                false,
                ColorDepth::NoColor,
            ));
            continue;
        }

        layers.push(build_layer(&writer));
    }

    let env_filter = tracing_subscriber::EnvFilter::try_new(filter.as_directive())?;
    let (env_filter_layer, raw) = tracing_subscriber::reload::Layer::new(env_filter);

    let subscriber = Registry::default().with(layers).with(env_filter_layer);

    let _ = tracing_log::LogTracer::init();
    tracing::subscriber::set_global_default(subscriber)?;

    Ok(TracingGuard {
        raw,
        filter,
        #[cfg(feature = "file")]
        worker_guards: file_guards,
        #[cfg(feature = "file")]
        log_paths,
    })
}

#[must_use = "dropping TracingGuard will release associated resources"]
pub struct TracingGuard {
    pub(crate) raw: ReloadHandle,
    pub(crate) filter: Filter,
    #[cfg(feature = "file")]
    pub(crate) worker_guards: Vec<writer::LogHandle>,
    #[cfg(feature = "file")]
    pub(crate) log_paths: Vec<PathBuf>,
}

impl std::fmt::Debug for TracingGuard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut d = f.debug_struct("TracingGuard");
        let _ = d.field("filter", &self.filter);
        #[cfg(feature = "file")]
        let _ = d.field("log_paths", &self.log_paths);
        #[cfg(feature = "file")]
        let _ = d.field("num_file_guards", &self.worker_guards.len());
        d.finish_non_exhaustive()
    }
}

impl TracingGuard {
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
        self.log_paths.first().map(PathBuf::as_path)
    }
}
