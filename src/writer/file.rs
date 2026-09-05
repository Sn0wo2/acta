use std::path::{Path, PathBuf};

use crate::Result;
use crate::config::Rotation;
pub type LogHandle = tracing_appender::non_blocking::WorkerGuard;

#[allow(clippy::single_call_fn)]
pub(crate) fn build_file_layer(
    path: &Path,
    rotation: Rotation,
) -> Result<(
    tracing_appender::non_blocking::NonBlocking,
    LogHandle,
    PathBuf,
)> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    rotate_log_file(path, rotation)?;
    let path = resolve_log_path(path);

    let (non_blocking, guard) = tracing_appender::non_blocking(tracing_appender::rolling::never(
        path.parent().unwrap_or(Path::new(".")),
        path.file_name().unwrap_or_default(),
    ));

    Ok((non_blocking, guard, path))
}

#[allow(clippy::module_name_repetitions)]
pub fn rotate_log_file(path: &Path, mode: Rotation) -> Result<()> {
    if !path.exists() {
        return Ok(());
    }
    match mode {
        Rotation::None => Ok(()),
        Rotation::Rename => {
            let renamed = path.with_extension(format!("{}.log", now_timestamp()));
            std::fs::rename(path, renamed)?;
            Ok(())
        }
        #[cfg(feature = "compress")]
        Rotation::Compress => {
            let timestamp = now_timestamp();
            let tmp_path = path.with_extension(format!("{timestamp}.log.tmp_compress"));
            std::fs::rename(path, &tmp_path)?;

            let orig_path_buf = path.to_path_buf();
            std::thread::spawn(move || {
                use flate2::Compression;
                use flate2::read::GzEncoder;
                use std::io::{BufWriter, Write};

                let gz_path = orig_path_buf.with_extension(format!("{timestamp}.log.gz"));
                let compress_res = (|| -> std::io::Result<()> {
                    let mut input = std::fs::File::open(&tmp_path)?;
                    let output = std::fs::File::create(&gz_path)?;
                    let mut encoder = GzEncoder::new(&mut input, Compression::default());
                    let mut buf_writer = BufWriter::new(output);
                    std::io::copy(&mut encoder, &mut buf_writer)?;
                    buf_writer.flush()?;
                    drop(buf_writer);
                    std::fs::remove_file(&tmp_path)?;
                    Ok(())
                })();

                if let Err(e) = compress_res {
                    let _unused = writeln!(
                        std::io::stderr(),
                        "async log compression failed for {}: {e}",
                        tmp_path.display()
                    );
                }
            });

            Ok(())
        }
    }
}

#[cfg_attr(not(feature = "compress"), allow(clippy::single_call_fn))]
fn now_timestamp() -> String {
    chrono::Local::now().format("%Y-%m-%d_%H-%M-%S").to_string()
}

pub fn resolve_log_path(path: &Path) -> PathBuf {
    match std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        Ok(_) => path.to_path_buf(),
        Err(_) => {
            let pid = std::process::id();
            let stem = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("latest");
            let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("log");
            path.with_file_name(format!("{stem}-{pid}.{ext}"))
        }
    }
}
