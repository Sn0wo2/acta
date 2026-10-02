#[cfg(feature = "file")]
use std::path::PathBuf;
#[cfg(feature = "file")]
use tracing_appender::non_blocking::WorkerGuard;

#[cfg(feature = "file")]
use crate::config::WriterTarget;
use crate::config::{ColorDepth, Config, Filter, Format, Writer};
use crate::fmt::Formatter;
use crate::writer;
use tracing_subscriber::Registry;
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::fmt::writer::BoxMakeWriter;
use tracing_subscriber::layer::Layered;
use tracing_subscriber::prelude::*;

pub(crate) type BoxedLayer = Box<dyn tracing_subscriber::Layer<Registry> + Send + Sync>;
pub(crate) type InnerSubscriber = Layered<Vec<BoxedLayer>, Registry>;
pub(crate) type ReloadHandle =
    tracing_subscriber::reload::Handle<tracing_subscriber::EnvFilter, InnerSubscriber>;

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
                .with_style(writer.style)
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

#[cfg(not(target_arch = "wasm32"))]
fn resolve_color_depth(writer: &Writer) -> ColorDepth {
    writer.color_depth.unwrap_or_else(|| {
        if writer.ansi {
            crate::utils::terminal::detect_color_depth(&writer.target)
        } else {
            ColorDepth::NoColor
        }
    })
}

#[cfg(target_arch = "wasm32")]
fn resolve_color_depth(writer: &Writer) -> ColorDepth {
    writer.color_depth.unwrap_or(if writer.ansi {
        ColorDepth::TrueColor
    } else {
        ColorDepth::NoColor
    })
}

pub fn build_layer(writer: &Writer) -> BoxedLayer {
    build_fmt_layer(
        writer,
        writer::make_writer(&writer.target),
        writer.ansi,
        resolve_color_depth(writer),
    )
}

pub fn init(config: impl Into<Config>) -> crate::Result<TracingGuard> {
    let Config { filter, writers } = config.into();
    let mut layers: Vec<BoxedLayer> = Vec::with_capacity(writers.len());

    #[cfg(feature = "file")]
    let (mut worker_guards, mut log_paths) = (Vec::new(), Vec::new());

    for writer in writers {
        let built = writer::build_writer(&writer.target)?;

        #[cfg(feature = "file")]
        if let Some((guard, path)) = built.file {
            worker_guards.push(guard);
            log_paths.push(path);
        }

        #[cfg(feature = "file")]
        let is_file = matches!(writer.target, WriterTarget::File(_));
        #[cfg(not(feature = "file"))]
        let is_file = false;

        let (ansi, color_depth) = if is_file {
            (false, ColorDepth::NoColor)
        } else {
            (writer.ansi, resolve_color_depth(&writer))
        };
        layers.push(build_fmt_layer(
            &writer,
            built.make_writer,
            ansi,
            color_depth,
        ));
    }
    let (env_filter_layer, raw) = tracing_subscriber::reload::Layer::new(
        tracing_subscriber::EnvFilter::try_new(filter.as_directive())?,
    );

    tracing_log::LogTracer::init()?;

    tracing::subscriber::set_global_default(
        Registry::default().with(layers).with(env_filter_layer),
    )?;

    Ok(TracingGuard {
        raw,
        filter,
        #[cfg(feature = "file")]
        worker_guards,
        #[cfg(feature = "file")]
        log_paths,
    })
}

#[must_use = "dropping TracingGuard will release associated resources"]
pub struct TracingGuard {
    pub(crate) raw: ReloadHandle,
    pub(crate) filter: Filter,
    #[cfg(feature = "file")]
    pub(crate) worker_guards: Vec<WorkerGuard>,
    #[cfg(feature = "file")]
    pub(crate) log_paths: Vec<PathBuf>,
}

impl std::fmt::Debug for TracingGuard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut d = f.debug_struct("TracingGuard");
        let _ = d.field("filter", &self.filter);
        #[cfg(feature = "file")]
        let _ = d
            .field("log_paths", &self.log_paths)
            .field("num_file_guards", &self.worker_guards.len());
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
