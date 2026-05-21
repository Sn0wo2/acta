#![allow(clippy::unwrap_used)]

use std::io;
use std::sync::Arc;

use super::*;
use crate::builder::{BoxedLayer, ReloadHandle, build_layer};
use crate::config::LayerConfig;
use tracing_subscriber::Registry;
use tracing_subscriber::layer::Layered;
use tracing_subscriber::prelude::*;

type TestSubscriber = Layered<
    tracing_subscriber::reload::Layer<tracing_subscriber::EnvFilter, builder::InnerSubscriber>,
    builder::InnerSubscriber,
>;

fn build_test_guard(level: Level, style: Style) -> (TracingGuard, TestSubscriber) {
    let filter = Filter::new(level);
    let env_filter =
        tracing_subscriber::EnvFilter::try_new(filter.as_directive()).unwrap_or_default();
    let (env_layer, raw): (_, ReloadHandle) = tracing_subscriber::reload::Layer::new(env_filter);

    let layers: Vec<BoxedLayer> = vec![
        tracing_subscriber::fmt::Layer::default()
            .with_writer(io::sink)
            .boxed(),
    ];

    let subscriber = Registry::default().with(layers).with(env_layer);

    let guard = TracingGuard {
        raw,
        filter,
        style: Arc::new(arc_swap::ArcSwap::new(Arc::new(style))),
        #[cfg(feature = "file")]
        worker_guard: None,
        #[cfg(feature = "file")]
        log_path: None,
    };

    (guard, subscriber)
}

#[test]
fn build_layer_all_variants() {
    let formats = [
        Format::Pretty(LayerConfig::pretty()),
        Format::Compact(LayerConfig::compact()),
        Format::Json(LayerConfig::json()),
    ];
    let targets = [WriterTarget::Stdout, WriterTarget::Stderr];

    for format in &formats {
        for target in &targets {
            let w = Writer {
                format: format.clone(),
                ansi: true,
                color_depth: None,
                show_path: true,
                show_spans: true,
                time_format: None,
                style: Style::default(),
                target: target.clone(),
            };
            let _layer = build_layer(&w);
        }
    }
}

#[test]
fn build_layer_no_ansi() {
    let w = Writer {
        ansi: false,
        ..Default::default()
    };
    let _layer = build_layer(&w);
}

#[test]
fn build_layer_custom_time() {
    let w = Writer {
        time_format: Some(String::from("%Y/%m/%d")),
        ..Default::default()
    };
    let _layer = build_layer(&w);
}

#[cfg(feature = "nerd")]
#[test]
fn build_layer_with_nerd_icons() {
    let w = Writer::default();
    let _layer = build_layer(&w);
}

#[test]
fn reload_handle_with_style_config() {
    let (mut handle, _sub) = build_test_guard(Level::Info, Style::default());
    handle.with_style(|s| s.theme = Theme::dracula());
    handle.with_style(|s| s.icons = Icons::UNICODE);
    handle.with_style(|s| s.labels = LevelLabels::SHORT);
}

#[test]
fn reload_handle_set_target_level_accepts_string() {
    let (mut handle, _sub) = build_test_guard(Level::Info, Style::default());
    let target = String::from("my_crate");
    assert!(handle.set_target_level(target, Level::Trace).is_ok());
}

#[test]
fn reload_handle_remove_nonexistent_target_level() {
    let (mut handle, _sub) = build_test_guard(Level::Info, Style::default());
    assert!(handle.remove_target_level("nonexistent_crate").is_ok());
}

#[test]
fn acta_error_display_lock_poisoned() {
    let msg = format!("{}", ActaError::LockPoisoned);
    assert!(msg.contains("log filter state lock poisoned"));
}

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
