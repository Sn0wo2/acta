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

For wasm32 targets, use `wasm-console` instead (see [WebAssembly](#webassembly)):

```toml
[dependencies]
acta = { version = "x.x.x", default-features = false, features = ["wasm-console"] }
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

Keep the returned guard alive for as long as logging is needed. Dropping it flushes and stops file logging.

## Features

| Feature        | Enabled by default | Description                                                                        |
| -------------- | ------------------ | ---------------------------------------------------------------------------------- |
| `unicode`      | Yes                | Unicode icon set, always available through `Icons::UNICODE`.                       |
| `file`         | Yes                | Enables `WriterTarget::File` and `FileConfig` through `tracing-appender`.          |
| `compress`     | No                 | Enables `Rotation::Compress` for gzip-compressing old log files.                   |
| `serde`        | No                 | Adds `Serialize` / `Deserialize` support for config types.                         |
| `nerd`         | No                 | Enables Nerd Font icons through `Icons::NERD`.                                     |
| `custom-async` | No                 | Enables Tokio-backed async console writers.                                        |
| `native-async` | No                 | Enables non-blocking console writers backed by `tracing-appender`. Implies `file`. |
| `async`        | No                 | Enables both `custom-async` and `native-async`.                                    |
| `wasm-console` | No                 | Enables console logging on wasm32 targets.                                         |

`file`, `compress`, `custom-async`, and `native-async` are not supported on wasm32 and fail to compile there; use `wasm-console` for those targets.

## Configuration

`Config::default()` uses:

- **Level**: `Level::Info`
- **Format**: `Format::Compact` (custom formatter with themes)
- **Writer**: `Writer::Stdout` with ANSI colors enabled
- **Color depth**: auto-detected from the terminal; `ColorDepth::TrueColor` on wasm
- **Path and span display**: enabled
- **File logging**: disabled

`init` accepts anything convertible into a `Config`: a `Level`, a `Filter`, a single `Writer`, a `Vec<Writer>`, or a full `Config`.

```rust
use acta::{init, Level, Result, Theme, Writer};

fn main() -> Result<()> {
    // Simplest: just pick a level.
    // let _guard = init(Level::Debug)?;

    // Or configure a writer fluently:
    let writer = Writer::stdout()
        .with_theme(Theme::tokyo_night())
        .with_time_format("%Y-%m-%d %H:%M:%S");
    let _guard = init(writer)?;

    Ok(())
}
```

Multiple writers with a custom filter go through the builder:

```rust
use acta::{init, Config, Level, Result, Writer};

fn main() -> Result<()> {
    let config = Config::builder()
        .level(Level::Debug)
        .with_writer(Writer::stdout().pretty())
        .with_writer(Writer::stderr().json().with_ansi(false))
        .build();

    let _guard = init(config)?;

    Ok(())
}
```

## Console formats

| Format                         | Description                                                        |
| ------------------------------ | ------------------------------------------------------------------ |
| `Format::Compact(LayerConfig)` | Default themed formatter with optional path and span display.      |
| `Format::Pretty(LayerConfig)`  | `tracing-subscriber` pretty formatter with file and line metadata. |
| `Format::Json(LayerConfig)`    | Flattened JSON events without ANSI colors.                         |

`LayerConfig` controls the metadata columns: `target`, `file`, and `line_number`; JSON output additionally honors `current_span`, `span_list`, and `flatten_event`. Convenience constructors are available: `LayerConfig::pretty()`, `LayerConfig::compact()`, and `LayerConfig::json()`.

## File logging

File logging is available with the `file` feature, which is enabled by default. File logs are written as flattened JSON events with ANSI colors disabled.

```rust
use acta::{init, Result, Writer};

fn main() -> Result<()> {
    let _guard = init(Writer::file("logs/app.log"))?;

    println!("Logging to {:?}", _guard.log_path());
    Ok(())
}
```

The `Rotation` can also be set through `FileConfig`:

```rust
use acta::{FileConfig, Rotation, Writer, WriterTarget};

let writer = Writer::default().with_target(WriterTarget::File(
    FileConfig::new("logs/app.log").with_rotation(Rotation::Rename),
));
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
use acta::{init, Filter, Result};

fn main() -> Result<()> {
    let _guard = init(Filter::from_directive("info,my_crate=debug,my_crate::db=trace"))?;
    Ok(())
}
```

Filters can also be built programmatically; `Level::Off` disables logging entirely.

```rust
use acta::{Config, Filter, Level};

let mut filter = Filter::new(Level::Info);
filter.with_target("my_crate", Level::Debug);
let _guard = init(filter)?;
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
use acta::Filter;

let directive = std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string());
let filter = Filter::from_directive(directive);
```

## Custom formatter

`Formatter` powers `Format::Compact` and can be customized through builder methods. Icons and labels are set through `Style`.

```rust
use acta::{Formatter, Icons, LevelLabels, Style, Theme};

let formatter = Formatter::new()
    .with_style(Style {
        theme: Theme::tokyo_night(),
        icons: Icons::UNICODE,
        labels: LevelLabels::SHORT,
    })
    .with_time_format("%H:%M:%S")
    .with_show_path(true)
    .with_show_spans(true);
```

The default path width is computed at acta's build time from its own source
tree — fine for a quick start, but if you want the column to fit **your**
project's source paths, add the `acta-build` helper to your build script:

```toml
[build-dependencies]
acta-build = "0.2"
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

## Color depth

Console output is rendered for the terminal's actual color capabilities. On native targets the depth is auto-detected; on wasm it defaults to `ColorDepth::TrueColor`. File writers always use `ColorDepth::NoColor`.

| Depth                   | Description                          |
| ----------------------- | ------------------------------------ |
| `ColorDepth::TrueColor` | 24-bit RGB colors.                   |
| `ColorDepth::Ansi256`   | 256-color palette, lossy downscale.  |
| `ColorDepth::Ansi16`    | 16-color palette, lossy downscale.   |
| `ColorDepth::NoColor`   | Plain text without escape sequences. |

Override the detection per writer:

```rust
use acta::{ColorDepth, Writer};

let writer = Writer::stdout().with_color_depth(ColorDepth::Ansi256);
```

`Writer::with_ansi(false)` disables escape sequences entirely, regardless of depth.

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
let long_labels = LevelLabels::LONG;
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

## Async console writers

With `custom-async`, `native-async`, or `async`, `Writer` gains async stdout and stderr variants: `Writer::async_stdout()` and `Writer::async_stderr()`, both using the default `AsyncMode`.

`AsyncMode::Custom` uses Tokio, so your application must run inside a Tokio runtime. If you use `#[tokio::main]`,
add Tokio as a direct dependency with the required runtime and macro features.

`buffer_size` sets the bounded-channel capacity — the number of log messages that may be queued before new ones
are dropped. Defaults to `DEFAULT_ASYNC_BUFFER_SIZE` (4096); with `serde` enabled the field may be omitted.

```rust
use acta::{init, AsyncMode, Result, Writer, WriterTarget};

#[tokio::main]
async fn main() -> Result<()> {
    let writer = Writer::default().with_target(WriterTarget::AsyncStdout(
        AsyncMode::Custom { buffer_size: 4096 },
    ));

    let _guard = init(writer)?;

    Ok(())
}
```

`AsyncMode::Native` uses `tracing-appender` non-blocking writers:

```rust
use acta::{init, AsyncMode, Result, Writer, WriterTarget};

fn main() -> Result<()> {
    let writer = Writer::default().with_target(WriterTarget::AsyncStdout(AsyncMode::Native));
    let _guard = init(writer)?;
    Ok(())
}
```

## WebAssembly

With the `wasm-console` feature, acta runs on `wasm32` and writes to the JavaScript console: `Writer::stdout()` maps to `console.log`, `Writer::stderr()` to `console.error`.

In a browser the ANSI escape sequences are converted to `%c` CSS styles so colors render correctly in DevTools; in non-browser JS runtimes such as Node or Cloudflare Workers, the ANSI sequences pass through raw to the terminal. Enable ANSI in browser-compatible output with `Writer::with_ansi(true)` (the default), and pick an explicit depth with `Writer::with_color_depth` if needed.

```rust
use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
fn main() {
    let _guard = acta::init(acta::Level::Info).expect("failed to initialize logging");

    tracing::info!("Hello from wasm!");
}
```

The guard can be dropped at the end of `main`; the global subscriber keeps logging. Store it in a `static` (for example through `OnceLock`) only if you plan to reload filters at runtime. The `file`, `compress`, `custom-async`, and `native-async` features fail to compile on wasm32 by design.
