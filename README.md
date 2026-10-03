# acta

[![Rust](https://img.shields.io/badge/rust-%3E%3D1.85-orange?style=flat-square&logo=rust&logoColor=white&labelColor=1a1b27)](https://www.rust-lang.org)
[![Edition](https://img.shields.io/badge/edition-2024-blue?style=flat-square&logo=rust&logoColor=white&labelColor=1a1b27)](https://doc.rust-lang.org/edition-guide/)
[![DeepWiki](https://img.shields.io/badge/DeepWiki-acta-2ea44f?style=flat-square&logo=gitbook&logoColor=white&labelColor=1a1b27)](https://deepwiki.com/Sn0wo2/acta)

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
use acta::prelude::*;

fn main() -> Result<()> {
    let _guard = init(Config::default())?;
    info!(user = "alice", "Hello, acta!");
    Ok(())
}
```

Defaults: compact stdout, `info` level, ANSI colors, paths and spans.
Keep the guard alive until logging is finished.

## Configuration

```rust
let config = Config::builder()
    .level(Level::Debug)
    .with_writer(
        Writer::stdout()
            .with_theme(Theme::tokyo_night())
            .with_time_format("%H:%M:%S")
            .with_show_path(false),
    )
    .with_writer(Writer::file("logs/app.log").json())
    .build();

let _guard = init(config)?;
```

`init` also accepts a `Filter`, a `Writer`, or a `Vec<Writer>`.
Choose `.compact()`, `.pretty()`, or `.json()` per writer.
Themes, icons and labels apply to compact output; file writers disable ANSI.

Color depth is detected automatically. Override it with
`.with_color_depth(ColorDepth::Ansi256)` or disable ANSI with `.with_ansi(false)`.

## Filters

```rust
let mut guard = init(Filter::from_directive("info,my_crate=debug"))?;
guard.set_level(Level::Debug)?;
guard.set_target_level("my_crate", Level::Trace)?;
guard.remove_target_level("my_crate")?;
```

Directives use [`EnvFilter` syntax](https://docs.rs/tracing-subscriber/latest/tracing_subscriber/filter/struct.EnvFilter.html).
`set_filter` replaces the entire filter. `RUST_LOG` is not read automatically.

## Style

```rust
const LABELS: LevelLabels = level_labels!(LevelLabels::LONG, LevelAlignment::Right);

let writer = Writer::stdout().with_style(Style {
    theme: Theme::one_dark(),
    icons: Icons::UNICODE,
    labels: LABELS,
});
```

`LONG`, `MEDIUM` and `SHORT` are already left-aligned; `LONG` is the style default.
`level_labels!(labels)` pads constant ASCII labels at compile time; pass
`LevelAlignment::Right` for right alignment. `LevelLabels::custom` accepts custom strings.

Themes: `acta`, `monokai`, `dracula`, `nord`, `catppuccin_mocha`, `gruvbox`,
`one_dark`, `tokyo_night`. Use `Theme::from_palette` and `Icons::custom` for custom styles.

For project-specific path widths, use [`acta-build`](https://docs.rs/acta-build)
with `Formatter::with_path_width`.

## File rotation

```rust
use acta::FileConfig;

let writer = Writer::default().with_target(WriterTarget::File(
    FileConfig::new("logs/app.log").with_rotation(Rotation::Rename),
));
```

`None` keeps the existing file; `Rename` archives it with a timestamp;
`Compress` archives it as gzip and requires the `compress` feature.
Rotation happens when the writer opens the file.

## Features

`unicode` and `file` are enabled by default.

| Feature        | Enables                                                     |
| -------------- | ----------------------------------------------------------- |
| `unicode`      | Currently has no effect; Unicode icons are always available |
| `file`         | File writers                                                |
| `compress`     | Gzip rotation; includes `file`                              |
| `serde`        | Config serialization; style is skipped                      |
| `nerd`         | `Icons::NERD`                                               |
| `custom-async` | Tokio console writers; requires an active runtime           |
| `native-async` | Non-blocking console writers; includes `file`               |
| `async`        | Both async backends                                         |
| `wasm-console` | JavaScript console output on wasm32                         |

Async targets use `WriterTarget::AsyncStdout` or `AsyncStderr` with
`AsyncMode::Custom { buffer_size: 4096 }` or `AsyncMode::Native`.

## WebAssembly

```toml
[dependencies]
acta = { version = "0.2", default-features = false, features = ["wasm-console"] }
```

Stdout maps to `console.log`; stderr maps to `console.error`.
File and async writers are not supported on wasm32.
