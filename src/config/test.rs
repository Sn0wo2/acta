#![allow(clippy::indexing_slicing)]

use super::*;
use crate::ActaError;
use std::io;

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
fn config_default() {
    let config = Config::default();
    assert_eq!(config.filter.as_directive(), "info");
    assert_eq!(config.writers.len(), 1);
    let w = &config.writers[0];
    assert!(matches!(w.format, Format::Compact(_)));
    assert!(w.ansi);
    assert!(w.show_path);
    assert!(w.show_spans);
    assert!(w.time_format.is_none());
}

#[test]
fn writer_default() {
    let w = Writer::default();
    assert!(matches!(w.format, Format::Compact(_)));
    assert!(w.ansi);
    assert!(w.show_path);
    assert!(w.show_spans);
    assert!(matches!(w.target, WriterTarget::Stdout));
}

#[test]
fn config_builder() {
    let cfg = Config::builder()
        .level(Level::Debug)
        .with_writer(Writer::default())
        .build();
    assert_eq!(cfg.filter.as_directive(), "debug");
    assert_eq!(cfg.writers.len(), 1);
}

#[test]
fn config_builder_multiple_writers() {
    let cfg = Config::builder()
        .level(Level::Info)
        .with_writer(Writer::stdout())
        .with_writer(Writer::stderr())
        .build();
    assert_eq!(cfg.writers.len(), 2);
}

#[test]
fn writer_chained_construction() {
    let w = Writer::stderr()
        .json()
        .with_ansi(false)
        .with_theme(Theme::monokai())
        .with_time_format("%H:%M");
    assert!(matches!(w.target, WriterTarget::Stderr));
    assert!(matches!(w.format, Format::Json(_)));
    assert!(!w.ansi);
    assert_eq!(w.time_format.as_deref(), Some("%H:%M"));
}

#[cfg(feature = "file")]
#[test]
fn writer_file_constructor() {
    let w = Writer::file("logs/app.log");
    assert!(matches!(w.target, WriterTarget::File(_)));
}

#[test]
fn format_shortcuts() {
    assert!(matches!(Format::pretty(), Format::Pretty(_)));
    assert!(matches!(Format::compact(), Format::Compact(_)));
    assert!(matches!(Format::json(), Format::Json(_)));
}

#[test]
fn config_from_filter() {
    let cfg: Config = Filter::new(Level::Debug).into();
    assert_eq!(cfg.filter.as_directive(), "debug");
    assert_eq!(cfg.writers.len(), 1);
}

#[test]
fn config_from_writer() {
    let cfg: Config = Writer::stderr().into();
    assert_eq!(cfg.filter.as_directive(), "info");
    assert!(matches!(cfg.writers[0].target, WriterTarget::Stderr));
}

#[test]
fn config_from_writer_vec() {
    let cfg: Config = vec![Writer::stdout(), Writer::stderr()].into();
    assert_eq!(cfg.writers.len(), 2);
}

#[test]
fn level_directives() {
    assert_eq!(Level::Error.as_directive(), "error");
    assert_eq!(Level::Warn.as_directive(), "warn");
    assert_eq!(Level::Info.as_directive(), "info");
    assert_eq!(Level::Debug.as_directive(), "debug");
    assert_eq!(Level::Trace.as_directive(), "trace");
    assert_eq!(Level::Off.as_directive(), "off");
}

#[test]
fn filter_from_directive() {
    let f = Filter::from_directive("info,my_crate=debug");
    assert_eq!(f.as_directive(), "info,my_crate=debug");
}

#[test]
fn filter_from_directive_with_extra_target() {
    let mut f = Filter::from_directive("info,bar=warn");
    f.with_target("foo", Level::Trace);
    let directive = f.as_directive();
    assert!(directive.starts_with("info,bar=warn"));
    assert!(directive.contains("foo=trace"));
}

#[test]
fn filter_builds_directive() {
    let filter = {
        let mut f = Filter::new(Level::Debug);
        f.with_target("my_crate", Level::Trace);
        f.with_target("my_crate::db", Level::Warn);
        f
    };

    let directive = filter.as_directive();
    assert!(directive.starts_with("debug,"));
    assert!(directive.contains("my_crate=trace"));
    assert!(directive.contains("my_crate::db=warn"));
    assert_eq!(directive.matches(',').count(), 2);
}

#[test]
fn filter_updates_targets() {
    let mut filter = Filter::new(Level::Info);
    filter.with_target("my_crate", Level::Debug);
    filter.with_target("my_crate", Level::Trace);

    assert_eq!(filter.as_directive(), "info,my_crate=trace");
    assert!(filter.remove_target("my_crate"));
    assert_eq!(filter.as_directive(), "info");
}

#[test]
fn rotation_default_is_none() {
    assert!(matches!(Rotation::default(), Rotation::None));
}

#[test]
fn filter_remove_target_exists() {
    let mut filter = Filter::new(Level::Info);
    filter.with_target("my_crate", Level::Debug);
    assert!(filter.remove_target("my_crate"));
}

#[test]
fn filter_remove_target_not_exists() {
    let mut filter = Filter::new(Level::Info);
    assert!(!filter.remove_target("nonexistent"));
}

#[test]
fn filter_default_is_info() {
    let filter = Filter::default();
    assert_eq!(filter.as_directive(), "info");
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
