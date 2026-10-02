use crate::config::{ColorDepth, WriterTarget};
use supports_color::Stream;

#[allow(clippy::single_call_fn)]
pub(crate) fn detect_color_depth(target: &WriterTarget) -> ColorDepth {
    if let Some(level) = supports_color::on_cached(match *target {
        WriterTarget::Stdout => Stream::Stdout,
        WriterTarget::Stderr => Stream::Stderr,
        #[cfg(feature = "file")]
        WriterTarget::File(_) => return ColorDepth::NoColor,
        #[cfg(acta_async)]
        WriterTarget::AsyncStdout(_) => Stream::Stdout,
        #[cfg(acta_async)]
        WriterTarget::AsyncStderr(_) => Stream::Stderr,
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
}
