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

```rust
use acta::Writer;

let errors_only = Writer::stderr().with_levels([
    tracing::Level::ERROR,
    tracing::Level::WARN,
]);
```

## Custom formatters and writers

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

## Structured events and spans

```rust
use acta::{Config, Event, Logger};
use tracing::Level;

fn main() -> acta::Result<()> {
    let logger = Logger::new(Config::builder().build())?;

    logger.event(&Event::new(Level::INFO, "service started"))?;

    let mut request = Event::new(Level::INFO, "request");
    request.fields.insert("path".into(), "/".into());
    let id = logger.create_span(&request)?;

    let mut done = Event::new(Level::INFO, "request finished");
    done.parent = Some(id);
    logger.event(&done)?;

    let mut fields = serde_json::Map::new();
    fields.insert("status".into(), 200.into());
    logger.record_span(id, &fields)?;
    logger.close_span(id)
}
```

### C and C++

Build with `cargo build -p acta-ffi --release`; the header is generated at
`crates/acta-ffi/include/acta_ffi.h`.

```c
#include "acta_ffi.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

int main(void) {
    const char *config = "{\"filter\":\"info\"}";
    acta_ffi_handle logger = 0;
    if (acta_logger_create((const uint8_t *)config, strlen(config), 64 * 1024, &logger)
        != ACTA_FFI_STATUS_OK) {
        fprintf(stderr, "acta: %s\n", acta_last_error());
        return 1;
    }

    acta_logger_log(logger, ACTA_FFI_LEVEL_INFO, (const uint8_t *)"hello", 5);

    uint8_t buffer[4096];
    uintptr_t written = 0;
    uint64_t dropped = 0;
    acta_logger_drain(logger, buffer, sizeof buffer, &written, &dropped);
    fwrite(buffer, 1, written, stdout);

    acta_logger_destroy(logger);
}
```

### Node.js / Bun

```sh
npm install acta-napi
```

Prebuilt addons ship for Linux, macOS and Windows on x64 and arm64.

```js
import { Logger } from "acta-napi";

const logger = new Logger({ filter: "info" });
logger.log(2, "hello from node"); // 0=error 1=warn 2=info 3=debug 4=trace
logger.event({ level: "info", message: "signed in", fields: { user: "alice" } });

const id = logger.createSpan({ level: "info", message: "request" });
logger.recordSpan(id, { status: 200 });
logger.closeSpan(id);

process.stdout.write(logger.drain());
logger.setFilter("debug");
logger.close();
```
### WebAssembly

The file and async writers features are not supported on wasm32:

- The [`acta-wasm`](https://docs.rs/acta-wasm) crate exposes the ABI Logger to
  JavaScript through wasm-bindgen:

  ```js
  import { Logger } from "./pkg/acta_wasm.js";

  const logger = new Logger('{"filter":"info"}', 64 * 1024);
  logger.log(2, "hello from wasm");
  const bytes = logger.drain(64 * 1024);
  ```

  Methods use camelCase on the JS side (`createSpan`, `recordSpan`, `closeSpan`).
  A config without `writers` buffers output for `drain()`; a `stdout` target maps
  to `console.log` and `stderr` to `console.error`.

- The `wasm-console` feature on the core crate enables the same console mapping
  for plain Rust code on wasm32:

  ```toml
  [dependencies]
  acta = { version = "0.6", default-features = false, features = ["wasm-console"] }
  ```

## Workspace

| Crate | Purpose |
| ----- | ------- |
| [`acta`](https://docs.rs/acta) | Core Rust library |
| [`acta-abi`](https://docs.rs/acta-abi) | Shared implementation behind all bindings; C ABI v2 |
| [`acta-ffi`](https://docs.rs/acta-ffi) | C foreign function interface (`staticlib` + `cdylib`) |
| [`acta-napi`](https://docs.rs/acta-napi) | Node.js and Bun N-API bindings (npm: `acta-napi`) |
| [`acta-wasm`](https://docs.rs/acta-wasm) | WebAssembly bindings (wasm-bindgen) |
| [`acta-build`](https://docs.rs/acta-build) | Build-time helper for path-column widths |
| `acta-debug` | Demo binary exercising the full API |

## Features

`file` is enabled by default.

| Feature        | Enables                                           |
| -------------- | ------------------------------------------------- |
| `file`         | File writers                                      |
| `compress`     | Gzip rotation; includes `file`                    |
| `serde`        | Config and event serialization                    |
| `nerd`         | `Icons::NERD`                                     |
| `custom-async` | Tokio console writers; requires an active runtime |
| `native-async` | Non-blocking console writers; includes `file`     |
| `async`        | Both async backends                               |
| `wasm-console` | JavaScript console output on wasm32               |

Async targets use `WriterTarget::AsyncStdout` or `AsyncStderr` with
`AsyncMode::Custom { buffer_size: 4096 }` or `AsyncMode::Native`.
Tokio console writers drain on `guard.flush()`; native and file writers flush
when the guard is dropped.

