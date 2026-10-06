#[cfg(any(not(acta_wasm_console), feature = "file"))]
use std::io;
#[cfg(feature = "file")]
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[cfg(feature = "file")]
use crate::config::Rotation;
#[cfg(feature = "file")]
use tracing_appender::non_blocking::WorkerGuard;

#[cfg(acta_async)]
use crate::config::AsyncMode;
use crate::config::WriterTarget;
use crate::config::{ColorDepth, Config, Format};
use crate::fmt::CustomFormatter;
use crate::fmt::stacktrace::Stacktrace;
use crate::writer::SharedWriter;
#[cfg(any(acta_wasm_console, acta_async))]
use crate::writer::Stream;
#[cfg(feature = "custom-async")]
use crate::writer::custom_async;
#[cfg(feature = "native-async")]
use crate::writer::native_async;
#[cfg(acta_wasm_console)]
use crate::writer::wasm;
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::fmt::writer::BoxMakeWriter;
use tracing_subscriber::layer::Layered;
use tracing_subscriber::prelude::*;
use tracing_subscriber::{EnvFilter, Registry};

type BoxedLayer = Box<dyn tracing_subscriber::Layer<Registry> + Send + Sync>;
type InnerSubscriber = Layered<Vec<BoxedLayer>, Registry>;
type ReloadHandle = tracing_subscriber::reload::Handle<EnvFilter, InnerSubscriber>;

pub fn init(config: Config) -> crate::Result<TracingGuard> {
    let Config { filter, writers } = config;
    let mut layers: Vec<BoxedLayer> = Vec::with_capacity(writers.len());

    #[cfg(feature = "file")]
    let mut worker_guards = Vec::new();
    #[cfg(feature = "custom-async")]
    let mut async_guards = Vec::new();

    for writer in writers {
        let make_writer = SharedWriter(match &writer.target {
            WriterTarget::Custom(writer) => Arc::clone(writer),
            #[cfg(not(acta_wasm_console))]
            WriterTarget::Stdout => Arc::new(BoxMakeWriter::new(io::stdout)),
            #[cfg(not(acta_wasm_console))]
            WriterTarget::Stderr => Arc::new(BoxMakeWriter::new(io::stderr)),
            #[cfg(acta_wasm_console)]
            WriterTarget::Stdout => {
                Arc::new(BoxMakeWriter::new(wasm::WasmWriter::new(Stream::Out)))
            }
            #[cfg(acta_wasm_console)]
            WriterTarget::Stderr => {
                Arc::new(BoxMakeWriter::new(wasm::WasmWriter::new(Stream::Err)))
            }
            #[cfg(feature = "file")]
            WriterTarget::File(config) => {
                if let Some(parent) = config.path.parent() {
                    std::fs::create_dir_all(parent)?;
                }

                if config.path.exists() {
                    let timestamp = chrono::Local::now()
                        .format("%Y-%m-%d_%H-%M-%S%.3f")
                        .to_string();
                    match config.rotation {
                        Rotation::None => {}
                        Rotation::Rename => {
                            let (archive, reservation) =
                                reserve_archive(&config.path, &format!("{timestamp}.log"))?;
                            drop(reservation);
                            std::fs::rename(&config.path, &archive)?;
                        }
                        #[cfg(feature = "compress")]
                        Rotation::Compress => {
                            let tmp_path = config
                                .path
                                .with_extension(format!("{timestamp}.log.compressing"));
                            std::fs::rename(&config.path, &tmp_path)?;

                            let (_, gz_file) =
                                reserve_archive(&config.path, &format!("{timestamp}.log.gz"))?;
                            let compress = move || {
                                use flate2::Compression;
                                use flate2::read::GzEncoder;
                                use std::io::{BufWriter, Write};

                                if let Err(e) = (|| -> io::Result<()> {
                                    let mut buf_writer = BufWriter::new(gz_file);

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

                let file = match std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&config.path)
                {
                    Ok(file) => file,
                    Err(_) => std::fs::OpenOptions::new().create(true).append(true).open(
                        config.path.with_file_name(format!(
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
                    )?,
                };

                let (w, guard) = tracing_appender::non_blocking(file);
                worker_guards.push(guard);
                Arc::new(BoxMakeWriter::new(w))
            }
            #[cfg(feature = "custom-async")]
            WriterTarget::AsyncStdout(AsyncMode::Custom { buffer_size }) => {
                let (w, guard) = custom_async::CustomAsyncWriter::new(Stream::Out, *buffer_size)?;
                async_guards.push(guard);
                Arc::new(BoxMakeWriter::new(w))
            }
            #[cfg(feature = "custom-async")]
            WriterTarget::AsyncStderr(AsyncMode::Custom { buffer_size }) => {
                let (w, guard) = custom_async::CustomAsyncWriter::new(Stream::Err, *buffer_size)?;
                async_guards.push(guard);
                Arc::new(BoxMakeWriter::new(w))
            }
            #[cfg(feature = "native-async")]
            WriterTarget::AsyncStdout(AsyncMode::Native) => {
                let (w, guard) = native_async::non_blocking(Stream::Out);
                worker_guards.push(guard);
                Arc::new(BoxMakeWriter::new(w))
            }
            #[cfg(feature = "native-async")]
            WriterTarget::AsyncStderr(AsyncMode::Native) => {
                let (w, guard) = native_async::non_blocking(Stream::Err);
                worker_guards.push(guard);
                Arc::new(BoxMakeWriter::new(w))
            }
        });

        let color_depth = match &writer.target {
            WriterTarget::Custom(_) => writer.color_depth.unwrap_or(ColorDepth::NoColor),
            #[cfg(feature = "file")]
            WriterTarget::File(_) => ColorDepth::NoColor,
            _ => {
                #[cfg(not(target_arch = "wasm32"))]
                {
                    writer.color_depth.unwrap_or_else(|| {
                        if let Some(level) = supports_color::on_cached(match &writer.target {
                            WriterTarget::Custom(_) => return ColorDepth::NoColor,
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
                    })
                }
                #[cfg(target_arch = "wasm32")]
                {
                    writer.color_depth.unwrap_or(ColorDepth::TrueColor)
                }
            }
        };
        let base = tracing_subscriber::fmt::Layer::default()
            .with_thread_ids(false)
            .with_thread_names(false)
            .with_span_events(FmtSpan::NONE)
            .with_writer(make_writer)
            .with_ansi(color_depth != ColorDepth::NoColor);

        let stacktrace_levels = writer
            .stacktrace
            .iter()
            .fold(0, |levels, level| match *level {
                tracing::Level::ERROR => levels | 1,
                tracing::Level::WARN => levels | 2,
                tracing::Level::INFO => levels | 4,
                tracing::Level::DEBUG => levels | 8,
                tracing::Level::TRACE => levels | 16,
            });

        layers.push(match writer.format {
            Format::Custom(formatter) => base
                .event_format(Stacktrace {
                    inner: CustomFormatter(formatter),
                    levels: stacktrace_levels,
                    flatten_event: None,
                })
                .boxed(),
            Format::Pretty(cfg) => base
                .pretty()
                .with_target(cfg.target)
                .with_file(cfg.file)
                .with_line_number(cfg.line_number)
                .map_event_format(|inner| Stacktrace {
                    inner,
                    levels: stacktrace_levels,
                    flatten_event: None,
                })
                .boxed(),
            Format::Compact(formatter) => base
                .event_format(formatter.with_color_depth(color_depth))
                .map_event_format(|inner| Stacktrace {
                    inner,
                    levels: stacktrace_levels,
                    flatten_event: None,
                })
                .boxed(),
            Format::Json(cfg) => base
                .json()
                .with_target(cfg.target)
                .with_file(cfg.file)
                .with_line_number(cfg.line_number)
                .with_current_span(cfg.current_span)
                .with_span_list(cfg.span_list)
                .flatten_event(cfg.flatten_event)
                .map_event_format(|inner| Stacktrace {
                    inner,
                    levels: stacktrace_levels,
                    flatten_event: Some(cfg.flatten_event),
                })
                .boxed(),
        });
    }
    let (env_filter_layer, raw) = tracing_subscriber::reload::Layer::new(filter);

    tracing_log::LogTracer::init()?;

    tracing::subscriber::set_global_default(
        Registry::default().with(layers).with(env_filter_layer),
    )?;

    Ok(TracingGuard {
        raw,
        #[cfg(feature = "file")]
        worker_guards,
        #[cfg(feature = "custom-async")]
        async_guards,
    })
}

#[cfg(feature = "file")]
#[cfg_attr(not(feature = "compress"), allow(clippy::single_call_fn))]
fn reserve_archive(base: &Path, extension: &str) -> io::Result<(PathBuf, std::fs::File)> {
    let mut attempt = 0;
    let mut path = base.with_extension(extension);
    loop {
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(file) => return Ok((path, file)),
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {
                attempt += 1;
                path = base.with_extension(format!("{extension}-{attempt}"));
            }
            Err(e) => return Err(e),
        }
    }
}

#[must_use = "dropping TracingGuard will release associated resources"]
pub struct TracingGuard {
    raw: ReloadHandle,
    #[cfg(feature = "file")]
    worker_guards: Vec<WorkerGuard>,
    #[cfg(feature = "custom-async")]
    async_guards: Vec<custom_async::CustomAsyncGuard>,
}

impl std::fmt::Debug for TracingGuard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut d = f.debug_struct("TracingGuard");
        #[cfg(feature = "file")]
        let _ = d.field("num_file_guards", &self.worker_guards.len());
        #[cfg(feature = "custom-async")]
        let _ = d.field("num_async_guards", &self.async_guards.len());
        d.finish_non_exhaustive()
    }
}

impl TracingGuard {
    #[cfg(feature = "custom-async")]
    pub fn flush(&self) {
        for guard in &self.async_guards {
            guard.flush();
        }
    }

    pub fn set_filter(&self, filter: EnvFilter) -> crate::Result<()> {
        self.raw.reload(filter)?;
        Ok(())
    }
}

#[cfg(test)]
mod test;
