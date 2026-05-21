# acta

[![Rust](https://img.shields.io/badge/rust-%3E%3D1.85-orange?style=flat-square&logo=rust&logoColor=white&labelColor=1a1b27)](https://www.rust-lang.org)
[![Edition](https://img.shields.io/badge/edition-2024-blue?style=flat-square&logo=rust&logoColor=white&labelColor=1a1b27)](https://doc.rust-lang.org/edition-guide/)
[![DeepWiki](https://img.shields.io/badge/DeepWiki-acta-2ea44f?style=flat-square&logo=gitbook&logoColor=white&labelColor=1a1b27)](https://deepwiki.com/Sn0wo2/acta)

[![CI](https://github.com/Sn0wo2/acta/actions/workflows/ci.yml/badge.svg)](https://github.com/Sn0wo2/acta/actions/workflows/ci.yml)
[![Release](https://github.com/Sn0wo2/acta/actions/workflows/release.yml/badge.svg)](https://github.com/Sn0wo2/acta/actions/workflows/release.yml)

> Make Tracing Great Again.

## Installation

```bash
cargo add acta
```

The default feature set enables Unicode console output and file logging.

```bash
cargo add acta --features serde,compress,nerd,async
```

## Quick start

Import the types you need directly:

```rust
use acta::{init, Config, Result};

fn main() -> Result<()> {
    let _guard = init(Config::default())?;

    tracing::info!("Hello, acta!");
    tracing::debug!(user = "alice", "User logged in");

    Ok(())
}
```

Or use the prelude for convenience:

```rust
use acta::prelude::*;

fn main() -> Result<()> {
    let _guard = init(Config::default())?;

    tracing::info!("Hello, acta!");
    tracing::debug!(user = "alice", "User logged in");

    Ok(())
}
```

Keep the returned guard alive for as long as logging is needed. Dropping it stops file logging.

## Features

| Feature        | Enabled by default | Description                                                                                              |
| -------------- | ------------------ | -------------------------------------------------------------------------------------------------------- |
| `unicode`      | Yes                | Uses the Unicode icon set unless `nerd` selects Nerd Font icons.                                         |
| `file`         | Yes                | Enables `init`, `TracingGuard`, and file logging through `tracing-appender`. |
| `compress`     | No                 | Enables `Rotation::Compress` for gzip-compressing old log files.                                         |
| `serde`        | No                 | Adds `Serialize` / `Deserialize` support for config types.                                               |
| `nerd`         | No                 | Enables Nerd Font icons through `Icons::NERD` and uses them by default.                         |
| `custom-async` | No                 | Enables Tokio-backed async console writers and exports `AsyncWriter` helpers.                            |
| `native-async` | No                 | Enables non-blocking console writers backed by `tracing-appender`.                                       |
| `async`        | No                 | Enables both `custom-async` and `native-async`.                                                          |

If you disable default features, `init` is unavailable unless the `file` feature is enabled.

## Configuration

`Config::default()` uses:

- **Level**: `Level::Info`
- **Format**: `Format::Compact` (custom formatter with themes)
- **Writer**: `Writer::Stdout` with ANSI colors enabled
- **Path and span display**: enabled
- **File logging**: disabled

```rust
use acta::{
    init, Format, LayerConfig, Level, Config, Result, Writer,
};

fn main() -> Result<()> {
    let config = Config {
        filter: Level::Debug.into(),
        writers: vec![Writer {
            format: Format::Compact(LayerConfig::compact()),
            ansi: true,
            show_path: true,
            show_spans: true,
            time_format: Some("%Y-%m-%d %H:%M:%S".to_string()),
            ..Default::default()
        }],
        ..Default::default()
    };

    let _guard = init(config)?;

    Ok(())
}
```

## Console formats

| Format            | Description                                                        |
| ----------------- | ------------------------------------------------------------------ |
| `Format::Compact(LayerConfig)` | Default themed formatter with optional path and span display.      |
| `Format::Pretty(LayerConfig)`  | `tracing-subscriber` pretty formatter with file and line metadata. |
| `Format::Json(LayerConfig)`    | Flattened JSON events without ANSI colors.                         |
## File logging

File logging is available with the `file` feature, which is enabled by default. File logs are written as flattened JSON
events.

```rust
use acta::{
    init, Format, LayerConfig, Level, Config, Result, Rotation, Writer, WriterTarget,
};
use std::path::PathBuf;

fn main() -> Result<()> {
    let config = Config {
        filter: Level::Info.into(),
        writers: vec![Writer {
            format: Format::Compact(LayerConfig::compact()),
            target: WriterTarget::File {
                path: PathBuf::from("logs/app.log"),
                rotation: Rotation::Rename,
            },
            ..Default::default()
        }],
        ..Default::default()
    };

    let _guard = init(config)?;

    Ok(())
}
```

Supported rotation modes:

| Mode                 | Description                                                                             |
| -------------------- | --------------------------------------------------------------------------------------- |
| `Rotation::None`     | Keeps the existing log file.                                                            |
| `Rotation::Rename`   | Renames the existing log file with a timestamp before opening a new one.                |
| `Rotation::Compress` | Compresses the existing log file to gzip before opening a new one. Requires `compress`. |

## Filter directives

acta uses `tracing-subscriber` `EnvFilter` directive syntax for startup filters and runtime reloads.

```rust
use acta::{Config, Filter};

let config = Config {
    filter: Filter::from_directive("info,my_crate=debug,my_crate::db=trace"),
    ..Default::default()
};
```

You can change filters after initialization directly through `TracingGuard`.

```rust
use acta::{init, Filter, Level, Config, Result};

fn main() -> Result<()> {
    let mut guard = init(Config::default())?;

    guard.set_level(Level::Debug)?;
    guard.set_target_level("my_crate", Level::Trace)?;
    guard.remove_target_level("my_crate")?;
    guard.set_filter(
        Filter::from_directive("warn,my_crate=debug"),
    )?;

    Ok(())
}
```

`RUST_LOG` is not read automatically. If you want to use it, pass its value into `Filter::from_directive`.

```rust
use acta::{Config, Filter};

let directive = std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string());
let config = Config {
    filter: Filter::from_directive(directive),
    ..Default::default()
};
```

## Custom formatter

`Formatter` powers `Format::Compact` and can be customized through builder methods.

```rust
use acta::{Formatter, Icons, LevelLabels, Theme};

let formatter = Formatter::new()
    .with_theme(Theme::tokyo_night())
    .with_icons(Icons::UNICODE)
    .with_labels(LevelLabels::DEFAULT)
    .with_time_format("%H:%M:%S")
    .with_show_path(true)
    .with_show_spans(true);
```

The default path width is computed at acta's build time from its own source
tree — fine for a quick start, but if you want the column to fit **your**
project's source paths, add the `acta-build` helper to your build script:

```toml
[build-dependencies]
acta-build = "0.1"
```

```rust
// build.rs
fn main() {
    let width = acta_build::walk_src_max_width("src", "src/");
    println!("cargo:rustc-env=ACTA_PATH_WIDTH={width}");
    println!("cargo:rerun-if-changed=src");
}
```

```rust
// in your code
let width: usize = env!("ACTA_PATH_WIDTH").parse().unwrap_or(40);
let formatter = Formatter::new().with_path_width(width);
```

## Themes

| Theme                       | Description      |
| --------------------------- | ---------------- |
| `Theme::acta()`             | Default          |
| `Theme::monokai()`          | Monokai          |
| `Theme::dracula()`          | Dracula          |
| `Theme::nord()`             | Nord             |
| `Theme::catppuccin_mocha()` | Catppuccin Mocha |
| `Theme::gruvbox()`          | Gruvbox          |
| `Theme::one_dark()`         | One Dark         |
| `Theme::tokyo_night()`      | Tokyo Night      |

Create custom themes from RGB values:

```rust
use acta::Theme;

let custom = Theme {
    accent:    (91, 206, 250),
    secondary: (245, 169, 184),
    text:      (255, 255, 255),
    error:     (255, 85, 85),
    warn:      (255, 200, 60),
    info:      (91, 206, 250),
    debug:     (245, 169, 184),
    trace:     (240, 240, 240),
};
```

## Icons and labels

```rust
use acta::{Icons, LevelLabels};

let unicode_icons = Icons::UNICODE;
let short_labels = LevelLabels::SHORT;
let long_labels = LevelLabels::DEFAULT;
```

With the `nerd` feature enabled:

```rust
use acta::Icons;

let nerd_icons = Icons::NERD;
```

Custom icons and labels:

```rust
use acta::{Icons, LevelLabels};

let custom_icons = Icons::custom("custom", "[", "]", "{", "}", "|", ">", "->", "·");
let custom_labels = LevelLabels::custom("ERR", "WRN", "INF", "DBG", "TRC");
```

## Runtime style reload

`TracingGuard` can reload themes, icons, and labels via `with_style` directly on the returned guard:

```rust
use acta::{init, Config, Theme, Result};

fn main() -> Result<()> {
    let mut guard = init(Config::default())?;

    // Reload the theme dynamically at runtime!
    guard.with_style(|s| s.theme = Theme::dracula());

    Ok(())
}
```

## Async console writers

With `custom-async`, `native-async`, or `async`, `Writer` gains async stdout and stderr variants.

`AsyncMode::Custom` uses Tokio, so your application must run inside a Tokio runtime. If you use `#[tokio::main]`,
add Tokio as a direct dependency with the required runtime and macro features.

```rust
use acta::{
    init, AsyncMode, Config, Result, Writer, WriterTarget,
};

#[tokio::main]
async fn main() -> Result<()> {
    let config = Config {
        writers: vec![Writer {
            target: WriterTarget::AsyncStdout(AsyncMode::Custom),
            ..Default::default()
        }],
        ..Default::default()
    };

    let _guard = init(config)?;

    Ok(())
}
```

`AsyncMode::Native` uses `tracing-appender` non-blocking writers.
