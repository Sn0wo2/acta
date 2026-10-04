#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::io;

use super::*;
use crate::config::{Filter, Level};
use tracing_subscriber::Registry;

type TestSubscriber = Layered<
    tracing_subscriber::reload::Layer<tracing_subscriber::EnvFilter, InnerSubscriber>,
    InnerSubscriber,
>;

fn build_test_guard(level: Level) -> (TracingGuard, TestSubscriber) {
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
        #[cfg(feature = "file")]
        worker_guards: Vec::new(),
        #[cfg(feature = "custom-async")]
        async_guards: Vec::new(),
    };
    (guard, subscriber)
}

#[test]
fn reload_handle_set_target_level_accepts_string() {
    let (mut handle, _sub) = build_test_guard(Level::Info);
    let target = String::from("my_crate");
    assert!(handle.set_target_level(target, Level::Trace).is_ok());
}

#[test]
fn reload_handle_remove_nonexistent_target_level() {
    let (mut handle, _sub) = build_test_guard(Level::Info);
    assert!(handle.remove_target_level("nonexistent_crate").is_ok());
}

#[test]
fn set_filter_with_raw_directive_updates_guard() {
    let (mut guard, subscriber) = build_test_guard(Level::Info);
    let filter = Filter::from_directive("info,my_crate=debug");
    guard
        .set_filter(filter)
        .expect("set_filter with raw directive should succeed");
    assert_eq!(
        guard.filter.as_directive(),
        "info,my_crate=debug",
        "guard.filter should reflect the raw directive applied via set_filter"
    );

    tracing::subscriber::with_default(subscriber, || {
        assert!(
            tracing::enabled!(tracing::Level::INFO),
            "info should be enabled with 'info' filter"
        );
        assert!(
            !tracing::enabled!(tracing::Level::TRACE),
            "trace should NOT be enabled with 'info' filter"
        );
    });
}

#[test]
fn set_level_after_filter_replaces_with_simple_level_directive() {
    let (mut guard, _subscriber) = build_test_guard(Level::Info);
    guard
        .set_filter(Filter::from_directive("info,my_crate=debug"))
        .expect("set_filter should succeed");
    guard
        .set_level(Level::Warn)
        .expect("set_level(Level::Warn) should succeed");
    assert_eq!(
        guard.filter.as_directive(),
        "warn",
        "set_level should replace the filter with a simple level directive"
    );
}

#[test]
fn set_target_level_after_raw_directive_adds_per_target_override() {
    let (mut guard, _subscriber) = build_test_guard(Level::Info);
    guard
        .set_filter(Filter::from_directive("info,my_crate=debug"))
        .expect("initial set_filter should succeed");
    guard
        .set_target_level("demo", Level::Trace)
        .expect("set_target_level should succeed after set_filter");
    let directive = guard.filter.as_directive();
    assert!(
        directive.contains("info"),
        "directive should still include base level after set_target_level: {directive}"
    );
    assert!(
        directive.contains("my_crate=debug"),
        "directive should retain existing per-target override after set_target_level: {directive}"
    );
    assert!(
        directive.contains("demo=trace"),
        "directive should include the new per-target override: {directive}"
    );
}

#[test]
fn set_filter_with_invalid_directive_returns_error() {
    let (mut guard, _sub) = build_test_guard(Level::Info);
    let filter = Filter::from_directive("foo=notalevel");
    let result = guard.set_filter(filter);
    assert!(
        result.is_err(),
        "set_filter with invalid directive should return Err"
    );
}
