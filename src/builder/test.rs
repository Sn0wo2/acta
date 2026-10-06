#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::io;

use super::*;

#[test]
fn set_filter_with_raw_directive_updates_guard() {
    let (env_layer, raw): (_, ReloadHandle) =
        tracing_subscriber::reload::Layer::new(EnvFilter::new("info"));
    let layers: Vec<BoxedLayer> = vec![
        tracing_subscriber::fmt::Layer::default()
            .with_writer(io::sink)
            .boxed(),
    ];
    let subscriber = Registry::default().with(layers).with(env_layer);
    let guard = TracingGuard {
        raw,
        #[cfg(feature = "file")]
        worker_guards: Vec::new(),
        #[cfg(feature = "custom-async")]
        async_guards: Vec::new(),
    };
    guard
        .set_filter(EnvFilter::try_new("info,my_crate=debug").unwrap())
        .expect("set_filter with raw directive should succeed");
    tracing::subscriber::with_default(subscriber, || {
        assert!(tracing::enabled!(tracing::Level::INFO));
        assert!(!tracing::enabled!(tracing::Level::TRACE));
    });
}
