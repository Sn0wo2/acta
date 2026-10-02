#[cfg(not(acta_wasm_console))]
use std::io;

#[cfg(feature = "file")]
use tracing_appender::non_blocking::WorkerGuard;

#[cfg(acta_async)]
use crate::config::AsyncMode;
use crate::config::WriterTarget;
use crate::config::{ColorDepth, Config, Filter, Format};
use crate::fmt::Formatter;
#[cfg(any(acta_wasm_console, acta_async))]
use crate::writer::Stream;
#[cfg(feature = "custom-async")]
use crate::writer::custom_async;
#[cfg(feature = "file")]
use crate::writer::file;
#[cfg(feature = "native-async")]
use crate::writer::native_async;
#[cfg(acta_wasm_console)]
use crate::writer::wasm;
use tracing_subscriber::Registry;
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::fmt::writer::BoxMakeWriter;
use tracing_subscriber::layer::Layered;
use tracing_subscriber::prelude::*;

pub(crate) type BoxedLayer = Box<dyn tracing_subscriber::Layer<Registry> + Send + Sync>;
pub(crate) type InnerSubscriber = Layered<Vec<BoxedLayer>, Registry>;
pub(crate) type ReloadHandle =
    tracing_subscriber::reload::Handle<tracing_subscriber::EnvFilter, InnerSubscriber>;

pub fn init(config: impl Into<Config>) -> crate::Result<TracingGuard> {
    let Config { filter, writers } = config.into();
    let mut layers: Vec<BoxedLayer> = Vec::with_capacity(writers.len());

    #[cfg(feature = "file")]
    let mut worker_guards = Vec::new();

    for writer in writers {
        let make_writer = match &writer.target {
            #[cfg(not(acta_wasm_console))]
            WriterTarget::Stdout => BoxMakeWriter::new(io::stdout),
            #[cfg(not(acta_wasm_console))]
            WriterTarget::Stderr => BoxMakeWriter::new(io::stderr),
            #[cfg(acta_wasm_console)]
            WriterTarget::Stdout => BoxMakeWriter::new(wasm::WasmWriter::new(Stream::Out)),
            #[cfg(acta_wasm_console)]
            WriterTarget::Stderr => BoxMakeWriter::new(wasm::WasmWriter::new(Stream::Err)),
            #[cfg(feature = "file")]
            WriterTarget::File(config) => {
                let (w, guard) = file::new(&config.path, config.rotation)?;
                worker_guards.push(guard);
                BoxMakeWriter::new(w)
            }
            #[cfg(feature = "custom-async")]
            WriterTarget::AsyncStdout(AsyncMode::Custom { buffer_size }) => BoxMakeWriter::new(
                custom_async::CustomAsyncWriter::new(Stream::Out, *buffer_size),
            ),
            #[cfg(feature = "custom-async")]
            WriterTarget::AsyncStderr(AsyncMode::Custom { buffer_size }) => BoxMakeWriter::new(
                custom_async::CustomAsyncWriter::new(Stream::Err, *buffer_size),
            ),
            #[cfg(feature = "native-async")]
            WriterTarget::AsyncStdout(AsyncMode::Native) => {
                BoxMakeWriter::new(native_async::NativeAsyncWriter::new(Stream::Out))
            }
            #[cfg(feature = "native-async")]
            WriterTarget::AsyncStderr(AsyncMode::Native) => {
                BoxMakeWriter::new(native_async::NativeAsyncWriter::new(Stream::Err))
            }
        };

        #[cfg(feature = "file")]
        let is_file = matches!(writer.target, WriterTarget::File(_));
        #[cfg(not(feature = "file"))]
        let is_file = false;

        let (ansi, color_depth) = if is_file {
            (false, ColorDepth::NoColor)
        } else {
            #[cfg(not(target_arch = "wasm32"))]
            let color_depth = writer.color_depth.unwrap_or_else(|| {
                if writer.ansi {
                    crate::utils::terminal::detect_color_depth(&writer.target)
                } else {
                    ColorDepth::NoColor
                }
            });
            #[cfg(target_arch = "wasm32")]
            let color_depth = writer.color_depth.unwrap_or(if writer.ansi {
                ColorDepth::TrueColor
            } else {
                ColorDepth::NoColor
            });
            (writer.ansi, color_depth)
        };
        let base = tracing_subscriber::fmt::Layer::default()
            .with_thread_ids(false)
            .with_thread_names(false)
            .with_span_events(FmtSpan::NONE)
            .with_writer(make_writer)
            .with_ansi(ansi);

        let layer = match &writer.format {
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
        };
        layers.push(layer);
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
    })
}

#[must_use = "dropping TracingGuard will release associated resources"]
pub struct TracingGuard {
    pub(crate) raw: ReloadHandle,
    pub(crate) filter: Filter,
    #[cfg(feature = "file")]
    pub(crate) worker_guards: Vec<WorkerGuard>,
}

impl std::fmt::Debug for TracingGuard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut d = f.debug_struct("TracingGuard");
        let _ = d.field("filter", &self.filter);
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
}

#[cfg(test)]
mod test;
