#![allow(clippy::indexing_slicing)]

use super::*;
use crate::ActaError;
use std::io;
use tracing_subscriber::EnvFilter;

#[test]
fn acta_error_display_io() {
    let inner = io::Error::new(io::ErrorKind::NotFound, "test error");
    let msg = format!("{}", ActaError::Io(inner));
    assert!(msg.contains("I/O error"));
}

#[test]
fn acta_error_from_io_error() {
    let io_err = io::Error::new(io::ErrorKind::PermissionDenied, "access denied");
    let error: ActaError = io_err.into();
    assert!(matches!(error, ActaError::Io(_)));
}

#[test]
fn config_builder_default() {
    let cfg = Config::builder().build();
    assert_eq!(cfg.filter.to_string(), "info");
    assert_eq!(cfg.writers.len(), 1);
    let w = &cfg.writers[0];
    assert!(matches!(w.format, Format::Compact(_)));
    assert!(matches!(w.target, WriterTarget::Stdout));
}

#[test]
fn writer_default() {
    let w = Writer::default();
    assert!(matches!(w.format, Format::Compact(_)));
    assert!(matches!(w.target, WriterTarget::Stdout));
}

#[test]
fn config_builder() {
    let cfg = Config::builder()
        .with_filter(EnvFilter::new("debug"))
        .with_writers([Writer::default()])
        .build();
    assert_eq!(cfg.filter.to_string(), "debug");
    assert_eq!(cfg.writers.len(), 1);
}

#[test]
fn config_builder_multiple_writers() {
    let cfg = Config::builder()
        .with_filter(EnvFilter::new("info"))
        .with_writers([Writer::stdout(), Writer::stderr()])
        .build();
    assert_eq!(cfg.writers.len(), 2);
}

#[test]
fn writer_chained_construction() {
    let w = Writer::stderr()
        .with_format(Format::Json(JsonOptions::default()))
        .with_color_depth(ColorDepth::Ansi256)
        .with_stacktrace(tracing::Level::ERROR);
    assert!(matches!(w.target, WriterTarget::Stderr));
    assert!(matches!(w.format, Format::Json(_)));
    assert_eq!(w.stacktrace, vec![tracing::Level::ERROR]);
}

#[cfg(feature = "file")]
#[test]
fn writer_file_constructor() {
    let w = Writer::file("logs/app.log");
    assert!(matches!(w.target, WriterTarget::File(_)));
}

#[test]
fn rotation_default_is_none() {
    assert!(matches!(Rotation::default(), Rotation::None));
}

#[test]
#[cfg(feature = "file")]
fn writer_file_target() {
    let w = Writer {
        target: WriterTarget::File(FileConfig::new("app.log").with_rotation(Rotation::Rename)),
        ..Default::default()
    };
    assert!(matches!(w.target, WriterTarget::File(_)));
}

#[test]
fn theme_presets_are_distinct() {
    let s1 = format!("{:?}", Theme::acta());
    let s2 = format!("{:?}", Theme::monokai());
    let s3 = format!("{:?}", Theme::dracula());
    assert_ne!(s1, s2);
    assert_ne!(s2, s3);
}

#[test]
fn theme_all_have_distinct_accent_colors() {
    let themes = [
        Theme::acta(),
        Theme::monokai(),
        Theme::dracula(),
        Theme::nord(),
        Theme::catppuccin_mocha(),
        Theme::gruvbox(),
        Theme::one_dark(),
        Theme::tokyo_night(),
    ];

    for (i, theme_i) in themes.iter().enumerate() {
        for theme_j in themes.iter().skip(i + 1) {
            assert_ne!(
                format!("{:?}", theme_i.accent),
                format!("{:?}", theme_j.accent)
            );
        }
    }
}

#[test]
fn theme_default_equals_acta() {
    assert_eq!(
        format!("{:?}", Theme::default()),
        format!("{:?}", Theme::acta())
    );
}

#[test]
fn level_labels_short() {
    let labels = LevelLabels::SHORT;
    assert_eq!(labels.error, "E");
    assert_eq!(labels.warn, "W");
    assert_eq!(labels.info, "I");
    assert_eq!(labels.debug, "D");
    assert_eq!(labels.trace, "T");
}

#[test]
fn level_labels_medium() {
    let labels = LevelLabels::MEDIUM;
    assert_eq!(labels.error, "ERR");
    assert_eq!(labels.warn, "WRN");
    assert_eq!(labels.info, "INF");
    assert_eq!(labels.debug, "DBG");
    assert_eq!(labels.trace, "TRC");
}

#[test]
fn level_labels_default_is_short() {
    assert_eq!(LevelLabels::default(), LevelLabels::SHORT);
}
