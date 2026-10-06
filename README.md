# acta

[![Rust](https://img.shields.io/badge/rust-%3E%3D1.88-orange?style=flat-square&logo=rust&logoColor=white&labelColor=1a1b27)](https://www.rust-lang.org)
[![Edition](https://img.shields.io/badge/edition-2024-blue?style=flat-square&logo=rust&logoColor=white&labelColor=1a1b27)](https://doc.rust-lang.org/edition-guide/)
[![Ask DeepWiki](https://deepwiki.com/badge.svg)](https://deepwiki.com/Sn0wo2/acta)

[![CI](https://github.com/Sn0wo2/acta/actions/workflows/ci.yml/badge.svg)](https://github.com/Sn0wo2/acta/actions/workflows/ci.yml)
[![Release](https://github.com/Sn0wo2/acta/actions/workflows/release.yml/badge.svg)](https://github.com/Sn0wo2/acta/actions/workflows/release.yml)

> Make Tracing Great Again.

---

## Install

```sh
cargo add acta
```

## Quick start

```rust
use acta::{Config, init};
use tracing::info;

fn main() -> acta::Result<()> {
    let _guard = init(Config::builder().build())?;
    info!(user = "alice", "Hello, acta!");
    Ok(())
}
```

Defaults: compact stdout, `info` level, ANSI colors, paths and spans.
Keep the guard alive until logging is finished.

## Configuration

```rust
use acta::{Config, Format, Formatter, JsonOptions, Style, Theme, Writer};
use tracing_subscriber::EnvFilter;

let config = Config::builder()
    .with_filter(EnvFilter::new("debug"))
    .with_writers([
        Writer::stdout().with_format(Format::Compact(
            Formatter::new()
                .with_style(Style {
                    theme: Theme::tokyo_night(),
                    ..Default::default()
                })
                .with_time_format("%H:%M:%S")
                .with_show_path(false),
        )),
        Writer::file("logs/app.log").with_format(Format::Json(JsonOptions::default())),
    ])
    .build();

let _guard = init(config)?;
```

`with_writers([])` explicitly disables all output. Built-in formats are `Format::Compact(Formatter)`, `Format::Pretty(PrettyOptions)` and `Format::Json(JsonOptions)`; the default format is compact.
Themes, icons and labels apply to compact output; file writers disable ANSI.

Color depth is detected automatically. Override it with
`.with_color_depth(ColorDepth::Ansi256)` or disable ANSI with
`.with_color_depth(ColorDepth::NoColor)`.

## Custom formatters and writers

`Format::custom` accepts implementations of
`tracing_subscriber::fmt::FormatEvent<Registry, DefaultFields>`.
`Writer::custom` and `WriterTarget::custom` accept implementations of
`tracing_subscriber::fmt::MakeWriter`, including closures returning an `std::io::Write`.
Both interfaces require `Send + Sync + 'static`, but not `Clone` or `Debug`.

```rust
use std::fmt::{self, Write as _};
use tracing::Event;
use tracing_subscriber::Registry;
use tracing_subscriber::fmt::{FmtContext, FormatEvent, FormatFields};
use tracing_subscriber::fmt::format::{DefaultFields, Writer};

struct MyFormatter;

impl FormatEvent<Registry, DefaultFields> for MyFormatter {
    fn format_event(
        &self,
        ctx: &FmtContext<'_, Registry, DefaultFields>,
        mut writer: Writer<'_>,
        event: &Event<'_>,
    ) -> fmt::Result {
        write!(writer, "[{}] ", event.metadata().level())?;
        ctx.field_format().format_fields(writer.by_ref(), event)?;
        writeln!(writer)
    }
}

fn main() -> acta::Result<()> {
    let writer = acta::Writer::custom(std::io::stderr)
        .with_format(acta::Format::custom(MyFormatter));
    let _guard = acta::init(acta::Config::builder().with_writers([writer]).build())?;
    tracing::info!(user = "alice", "Hello!");
    Ok(())
}
```

Custom targets work with any format and default to no ANSI; use `with_color_depth`
to override this. Metadata-aware writers retain their `make_writer_for` behavior.
Custom formatters retain the configured text stacktrace behavior.
Custom formatter and target variants are code-only: serde rejects serialization
and deserialization of these variants. Custom writer flushing and background worker
lifetimes are managed by the supplied implementation, not `TracingGuard`.

## Filters

```rust
use acta::{Config, init};
use tracing_subscriber::EnvFilter;

let guard = init(Config::builder()
    .with_filter(EnvFilter::new("info,my_crate=debug"))
    .build())?;

guard.set_filter(EnvFilter::new("info,my_crate=debug,other=trace"))?;
```

Directives use [`EnvFilter` syntax](https://docs.rs/tracing-subscriber/latest/tracing_subscriber/filter/struct.EnvFilter.html).
`set_filter` replaces the entire filter at runtime. `RUST_LOG` is not read automatically.

## Style

```rust
use acta::{Format, Formatter, Icons, LevelAlignment, LevelLabels, Style, Theme, Writer};
use acta::level_labels;

const LABELS: LevelLabels = level_labels!(LevelLabels::LONG, LevelAlignment::Right);

let writer = Writer::stdout().with_format(Format::Compact(
    Formatter::new().with_style(Style {
        theme: Theme::one_dark(),
        icons: Icons::UNICODE,
        labels: LABELS,
    }),
));
```

`LONG`, `MEDIUM` and `SHORT` are already left-aligned; `LONG` is the style default.
`level_labels!(labels)` pads constant ASCII labels at compile time; pass
`LevelAlignment::Right` for right alignment. `LevelLabels::custom` accepts custom strings.

Themes: `acta`, `monokai`, `dracula`, `nord`, `catppuccin_mocha`, `gruvbox`,
`one_dark`, `tokyo_night`. Custom themes are plain structs — construct `Theme`
from its public `(r, g, b)` color fields. `Icons::custom` defines custom icon sets.

For project-specific path widths, use [`acta-build`](https://docs.rs/acta-build)
with `Formatter::with_path_width`.

## File rotation

```rust
use acta::{FileConfig, Rotation, Writer, WriterTarget};

let writer = Writer::stdout().with_target(WriterTarget::File(
    FileConfig::new("logs/app.log").with_rotation(Rotation::Rename),
));
```

`None` keeps the existing file; `Rename` archives it with a timestamp;
`Compress` archives it as gzip and requires the `compress` feature.
Rotation happens when the writer opens the file.

## Features

`file` is enabled by default.

| Feature        | Enables                                           |
| -------------- | ------------------------------------------------- |
| `file`         | File writers                                      |
| `compress`     | Gzip rotation; includes `file`                    |
| `serde`        | Config serialization; style is skipped            |
| `nerd`         | `Icons::NERD`                                     |
| `custom-async` | Tokio console writers; requires an active runtime |
| `native-async` | Non-blocking console writers; includes `file`     |
| `async`        | Both async backends                               |
| `wasm-console` | JavaScript console output on wasm32               |

Async targets use `WriterTarget::AsyncStdout` or `AsyncStderr` with
`AsyncMode::Custom { buffer_size: 4096 }` or `AsyncMode::Native`.
Tokio console writers drain on `guard.flush()`; native and file writers flush
when the guard is dropped.

## WebAssembly

```toml
[dependencies]
acta = { version = "0.x", default-features = false, features = ["wasm-console"] }
```

Stdout maps to `console.log`; stderr maps to `console.error`.
File and async writers are not supported on wasm32.
