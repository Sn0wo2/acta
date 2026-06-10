use std::io;

use super::*;
use crate::config::LayerConfig;
use tracing_subscriber::layer::Layered;
use tracing_subscriber::prelude::*;

pub type SubscriberWithBoth = Layered<
    tracing_subscriber::reload::Layer<tracing_subscriber::EnvFilter, builder::InnerSubscriber>,
    builder::InnerSubscriber,
>;

pub fn build_reload_filter(
    level: Level,
    style: Style,
) -> (
    tracing_subscriber::reload::Layer<builder::FmtLayer, tracing_subscriber::Registry>,
    TracingGuard,
    SubscriberWithBoth,
) {
    let stdout_layer = Some(
        tracing_subscriber::fmt::Layer::default()
            .with_writer(io::sink)
            .boxed(),
    );
    let stderr_layer = None;
    let async_stdout_layer = None;
    let async_stderr_layer = None;
    let file_layer = None;

    let inner_subscriber = tracing_subscriber::Registry::default()
        .with(stdout_layer)
        .with(stderr_layer)
        .with(async_stdout_layer)
        .with(async_stderr_layer)
        .with(file_layer);

    let filter = Filter::new(level);
    let env_filter =
        tracing_subscriber::EnvFilter::try_new(filter.as_directive()).unwrap_or_default();
    let (env_layer, env_handle) = tracing_subscriber::reload::Layer::new(env_filter);
    let subscriber = inner_subscriber.with(env_layer);

    let shared_state = std::sync::Arc::new(arc_swap::ArcSwap::new(std::sync::Arc::new(style)));
    let reload_handle = TracingGuard {
        raw: builder::ReloadHandle(env_handle),
        filter,
        style: shared_state,
        #[cfg(feature = "file")]
        worker_guard: None,
        #[cfg(feature = "file")]
        log_path: None,
    };

    let console_layer = tracing_subscriber::fmt::Layer::default()
        .with_writer(io::sink)
        .boxed();
    let (layer, _) = tracing_subscriber::reload::Layer::new(console_layer);
    (layer, reload_handle, subscriber)
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
            let _layer = build_layer::<tracing_subscriber::Registry>(&w);
        }
    }
}

#[test]
fn build_layer_no_ansi() {
    let w = Writer {
        ansi: false,
        ..Default::default()
    };
    let _layer = build_layer::<tracing_subscriber::Registry>(&w);
}

#[test]
fn build_layer_custom_time() {
    let w = Writer {
        time_format: Some(String::from("%Y/%m/%d")),
        ..Default::default()
    };
    let _layer = build_layer::<tracing_subscriber::Registry>(&w);
}

#[cfg(feature = "nerd")]
#[test]
fn build_layer_with_nerd_icons() {
    let w = Writer::default();
    let _layer = build_layer::<tracing_subscriber::Registry>(&w);
}

#[test]
fn reload_handle_with_style_config() {
    let style = Style::default();
    let (_layer, mut handle, _subscriber) = build_reload_filter(Level::Info, style);
    handle.with_style(|s| s.theme = Theme::dracula());
    handle.with_style(|s| s.icons = Icons::UNICODE);
    handle.with_style(|s| s.labels = LevelLabels::SHORT);
}

#[test]
fn reload_handle_set_target_level_accepts_string() {
    let (_layer, mut handle, _subscriber) = build_reload_filter(Level::Info, Style::default());
    let target = String::from("my_crate");
    assert!(handle.set_target_level(target, Level::Trace).is_ok());
}

#[test]
fn reload_handle_remove_nonexistent_target_level() {
    let (_layer, mut handle, _subscriber) = build_reload_filter(Level::Info, Style::default());
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
