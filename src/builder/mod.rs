#[cfg(any(not(acta_wasm_console), feature = "file"))]
use std::io;
#[cfg(feature = "file")]
use std::path::Path;

#[cfg(feature = "file")]
use crate::config::Rotation;
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
#[cfg(feature = "native-async")]
use crate::writer::native_async;
#[cfg(acta_wasm_console)]
use crate::writer::wasm;
use tracing_subscriber::Registry;
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::fmt::writer::BoxMakeWriter;
use tracing_subscriber::layer::Layered;
use tracing_subscriber::prelude::*;

type BoxedLayer = Box<dyn tracing_subscriber::Layer<Registry> + Send + Sync>;
type InnerSubscriber = Layered<Vec<BoxedLayer>, Registry>;
type ReloadHandle =
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
                if let Some(parent) = config.path.parent() {
                    std::fs::create_dir_all(parent)?;
                }

                if config.path.exists() {
                    match config.rotation {
                        Rotation::None => {}
                        Rotation::Rename => {
                            std::fs::rename(
                                &config.path,
                                config.path.with_extension(format!(
                                    "{}.log",
                                    chrono::Local::now().format("%Y-%m-%d_%H-%M-%S")
                                )),
                            )?;
                        }
                        #[cfg(feature = "compress")]
                        Rotation::Compress => {
                            let new_timestamp =
                                chrono::Local::now().format("%Y-%m-%d_%H-%M-%S").to_string();
                            let tmp_path = config
                                .path
                                .with_extension(format!("{new_timestamp}.log.compressing"));
                            std::fs::rename(&config.path, &tmp_path)?;

                            let gz_path = config
                                .path
                                .with_extension(format!("{new_timestamp}.log.gz"));
                            let compress = move || {
                                use flate2::Compression;
                                use flate2::read::GzEncoder;
                                use std::io::{BufWriter, Write};

                                if let Err(e) = (|| -> io::Result<()> {
                                    let mut buf_writer =
                                        BufWriter::new(std::fs::File::create(&gz_path)?);

                                    io::copy(
                                        &mut GzEncoder::new(
                                            std::fs::File::open(&tmp_path)?,
                                            Compression::default(),
                                        ),
                                        &mut buf_writer,
                                    )?;
                                    buf_writer.flush()?;
                                    drop(buf_writer);

                                    std::fs::remove_file(&tmp_path)?;
                                    Ok(())
                                })() {
                                    let _unused = writeln!(
                                        io::stderr(),
                                        "async log compression failed for {}: {e}",
                                        tmp_path.display()
                                    );
                                }
                            };

                            #[cfg(feature = "custom-async")]
                            {
                                if let Ok(handle) = tokio::runtime::Handle::try_current() {
                                    handle.spawn_blocking(compress);
                                } else {
                                    std::thread::spawn(compress);
                                }
                            }

                            #[cfg(not(feature = "custom-async"))]
                            {
                                std::thread::spawn(compress);
                            }
                        }
                    }
                }

                let path = match std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&config.path)
                {
                    Ok(_) => config.path.clone(),
                    Err(_) => config.path.with_file_name(format!(
                        "{}-{}.{}",
                        config
                            .path
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or("latest"),
                        std::process::id(),
                        config
                            .path
                            .extension()
                            .and_then(|s| s.to_str())
                            .unwrap_or("log")
                    )),
                };

                let (w, guard) = tracing_appender::non_blocking(tracing_appender::rolling::never(
                    path.parent().unwrap_or(Path::new(".")),
                    path.file_name().unwrap_or_default(),
                ));
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

        let (ansi, color_depth) = match &writer.target {
            #[cfg(feature = "file")]
            WriterTarget::File(_) => (false, ColorDepth::NoColor),
            _ => {
                #[cfg(not(target_arch = "wasm32"))]
                let color_depth = writer.color_depth.unwrap_or_else(|| {
                    if !writer.ansi {
                        return ColorDepth::NoColor;
                    }

                    if let Some(level) = supports_color::on_cached(match &writer.target {
                        WriterTarget::Stdout => supports_color::Stream::Stdout,
                        WriterTarget::Stderr => supports_color::Stream::Stderr,
                        #[cfg(feature = "file")]
                        WriterTarget::File(_) => return ColorDepth::NoColor,
                        #[cfg(acta_async)]
                        WriterTarget::AsyncStdout(_) => supports_color::Stream::Stdout,
                        #[cfg(acta_async)]
                        WriterTarget::AsyncStderr(_) => supports_color::Stream::Stderr,
                    }) {
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
                });
                #[cfg(target_arch = "wasm32")]
                let color_depth = writer.color_depth.unwrap_or(if writer.ansi {
                    ColorDepth::TrueColor
                } else {
                    ColorDepth::NoColor
                });
                (writer.ansi, color_depth)
            }
        };
        let base = tracing_subscriber::fmt::Layer::default()
            .with_thread_ids(false)
            .with_thread_names(false)
            .with_span_events(FmtSpan::NONE)
            .with_writer(make_writer)
            .with_ansi(ansi && color_depth != ColorDepth::NoColor);

        layers.push(match &writer.format {
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
                    formatter = formatter.with_time_format(tf);
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
        });
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
        self.raw.reload(tracing_subscriber::EnvFilter::try_new(
            filter.as_directive(),
        )?)?;
        self.filter = filter;
        Ok(())
    }

    pub fn set_level(&mut self, level: crate::config::Level) -> crate::Result<()> {
        self.set_filter(Filter::new(level))
    }

    pub fn set_target_level(
        &mut self,
        target: impl Into<compact_str::CompactString>,
        level: crate::config::Level,
    ) -> crate::Result<()> {
        let mut filter = self.filter.clone();
        filter.with_target(target, level);
        self.set_filter(filter)
    }

    pub fn remove_target_level(&mut self, target: &str) -> crate::Result<()> {
        let mut filter = self.filter.clone();
        filter.remove_target(target);
        self.set_filter(filter)
    }
}

#[cfg(test)]
mod test;
