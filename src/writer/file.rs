use std::path::Path;

use tracing_appender::non_blocking::{NonBlocking, WorkerGuard};

use crate::Result;
use crate::config::Rotation;

#[allow(clippy::single_call_fn)]
pub(crate) fn new(path: &Path, rotation: Rotation) -> Result<(NonBlocking, WorkerGuard)> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    if path.exists() {
        let new_timestamp = chrono::Local::now().format("%Y-%m-%d_%H-%M-%S").to_string();

        match rotation {
            Rotation::None => {}
            Rotation::Rename => {
                std::fs::rename(path, path.with_extension(format!("{new_timestamp}.log")))?;
            }
            #[cfg(feature = "compress")]
            Rotation::Compress => {
                let tmp_path = path.with_extension(format!("{new_timestamp}.log.compressing"));
                std::fs::rename(path, &tmp_path)?;

                let orig_path_buf = path.to_path_buf();

                let compress = move || {
                    use flate2::Compression;
                    use flate2::read::GzEncoder;
                    use std::io::{BufWriter, Write};

                    let gz_path = orig_path_buf.with_extension(format!("{new_timestamp}.log.gz"));

                    if let Err(e) = (|| -> std::io::Result<()> {
                        let mut input = std::fs::File::open(&tmp_path)?;
                        let output = std::fs::File::create(&gz_path)?;
                        let mut encoder = GzEncoder::new(&mut input, Compression::default());
                        let mut buf_writer = BufWriter::new(output);

                        std::io::copy(&mut encoder, &mut buf_writer)?;
                        buf_writer.flush()?;
                        drop(buf_writer);

                        std::fs::remove_file(&tmp_path)?;
                        Ok(())
                    })() {
                        let _unused = writeln!(
                            std::io::stderr(),
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
        .open(path)
    {
        Ok(_) => path.to_path_buf(),
        Err(_) => {
            let stem = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("latest");
            let pid = std::process::id();
            let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("log");
            path.with_file_name(format!("{stem}-{pid}.{ext}"))
        }
    };

    let (writer, guard) = tracing_appender::non_blocking(tracing_appender::rolling::never(
        path.parent().unwrap_or(Path::new(".")),
        path.file_name().unwrap_or_default(),
    ));

    Ok((writer, guard))
}
