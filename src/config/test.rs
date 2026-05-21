#![allow(clippy::indexing_slicing)]

use super::*;

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
fn config_builder_filter() {
    let cfg = Config::builder()
        .filter(Filter::from_directive("info,my_crate=debug"))
        .build();
    assert_eq!(cfg.filter.as_directive(), "info,my_crate=debug");
}

#[test]
fn config_builder_multiple_writers() {
    let cfg = Config::builder()
        .level(Level::Info)
        .with_writer(Writer {
            target: WriterTarget::Stdout,
            ..Default::default()
        })
        .with_writer(Writer {
            target: WriterTarget::Stderr,
            ..Default::default()
        })
        .build();
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
fn filter_from_level() {
    let filter: Filter = Level::Warn.into();
    assert_eq!(filter.as_directive(), "warn");
}

#[test]
fn filter_from_level_debug() {
    let filter: Filter = Level::Debug.into();
    assert_eq!(filter.as_directive(), "debug");
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
        target: WriterTarget::File {
            path: PathBuf::from("app.log"),
            rotation: Rotation::Rename,
        },
        ..Default::default()
    };
    assert!(matches!(w.target, WriterTarget::File { .. }));
}
