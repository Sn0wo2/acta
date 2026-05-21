#![allow(clippy::print_stdout, clippy::print_stderr, clippy::unwrap_used)]
use std::sync::LazyLock;

use acta::{
    Config, Filter, Format, Icons, LayerConfig, Level, LevelLabels, Rotation, Style, Theme, Writer,
    WriterTarget, build_layer, init, rotate_log_file,
};
use smallvec::{SmallVec, smallvec};
use tracing_subscriber::prelude::*;

fn section(title: &str) {
    let w: usize = 64;
    let pad = (w.saturating_sub(title.len())) / 2;
    println!(
        "\n┌{}┐\n|{:>pad$}{}{:pad$}|\n└{}┘",
        "─".repeat(w),
        "",
        title,
        "",
        "─".repeat(w)
    );
}

macro_rules! log {
    (sub, $msg:expr) => {
        println!("[-] {}", $msg)
    };
    (info, $msg:expr) => {
        println!("[+] {}", $msg)
    };
    (success, $msg:expr) => {
        println!("[√] {}", $msg)
    };
    (fail, $msg:expr) => {
        println!("[X] {}", $msg)
    };
    (pad, $msg:expr) => {
        println!("   · {}", $msg)
    };
}

fn run_with(w: &Writer, f: impl FnOnce()) {
    let layer = build_layer(w);
    let subscriber = tracing_subscriber::registry().with(layer);
    tracing::subscriber::with_default(subscriber, f);
}

fn emit_demo(label: &str) {
    tracing::info!("{label}: info");
    tracing::warn!(user = "alice", count = 42, "{label}: warn");
    tracing::error!(code = 500, "{label}: error");
}

fn emit_all_levels() {
    tracing::error!(code = 500, "error: crash");
    tracing::warn!(threshold = 0.9, "warn: resource limit");
    tracing::info!(users = 42, "info: normal");
    tracing::debug!(query = "SELECT *", took_ms = 3, "debug: query");
    tracing::trace!(state = "idle", "trace: idle");
}

fn emit_spans() {
    tracing::info!("no span");
    let _a = tracing::info_span!("layer1").entered();
    tracing::info!("span [layer1]");
    let _b = tracing::info_span!("layer2", depth = 2).entered();
    tracing::warn!("span [layer1 > layer2{{depth=2}}]");
    let _c = tracing::debug_span!("layer3").entered();
    tracing::error!("span [layer1 > layer2 > layer3]");
}

static ICONS: LazyLock<SmallVec<[(&str, Icons); 3]>> = LazyLock::new(|| {
    smallvec![
        ("unicode", Icons::UNICODE),
        (
            "no-icons",
            Icons::custom("no-icons", "", "", "", "", "", "", "", "")
        ),
        ("nerd", Icons::NERD),
    ]
});

static THEMES: LazyLock<SmallVec<[(&str, Theme); 8]>> = LazyLock::new(|| {
    smallvec![
        ("acta", Theme::acta()),
        ("monokai", Theme::monokai()),
        ("dracula", Theme::dracula()),
        ("nord", Theme::nord()),
        ("catppuccin_mocha", Theme::catppuccin_mocha()),
        ("gruvbox", Theme::gruvbox()),
        ("one_dark", Theme::one_dark()),
        ("tokyo_night", Theme::tokyo_night()),
    ]
});

fn main() {
    section("FORMAT × ICON");
    let formats: &[(&str, Format)] = &[
        ("compact", Format::Compact(LayerConfig::compact())),
        ("pretty", Format::Pretty(LayerConfig::pretty())),
        ("json", Format::Json(LayerConfig::json())),
    ];
    for (fmt_name, format) in formats {
        for (icon_name, icons) in &*ICONS {
            let style = Style {
                icons: *icons,
                ..Default::default()
            };
            let ansi = !matches!(format, Format::Json(_));
            let w = Writer {
                style,
                format: format.clone(),
                ansi,
                target: WriterTarget::Stdout,
                show_path: false,
                show_spans: false,
                ..Default::default()
            };
            log!(sub, &format!("{fmt_name} + {icon_name}"));
            run_with(&w, emit_all_levels);
        }
    }

    section("THEMES");
    log!(sub, "Palette overview");
    for (name, t) in &*THEMES {
        println!(
            "    {name:<22} accent={:?}  secondary={:?}  text={:?}",
            t.accent, t.secondary, t.text
        );
    }

    log!(sub, "Live preview per theme");
    for (name, theme) in &*THEMES {
        let w = Writer {
            style: Style {
                theme: *theme,
                ..Default::default()
            },
            format: Format::Compact(LayerConfig::compact()),
            target: WriterTarget::Stdout,
            show_path: false,
            show_spans: false,
            ..Default::default()
        };
        println!("  [{name}]");
        run_with(&w, || emit_demo(name));
    }

    section("ALL LEVELS");
    for (icon_name, icons) in &*ICONS {
        log!(sub, icon_name);
        run_with(
            &Writer {
                style: Style {
                    icons: *icons,
                    theme: Theme::one_dark(),
                    ..Default::default()
                },
                format: Format::Compact(LayerConfig::compact()),
                target: WriterTarget::Stdout,
                show_path: false,
                show_spans: false,
                ..Default::default()
            },
            emit_all_levels,
        );
    }

    section("LABELS");
    for (label, labels) in [
        ("short", LevelLabels::SHORT),
        ("long", LevelLabels::DEFAULT),
    ] {
        log!(sub, label);
        run_with(
            &Writer {
                style: Style {
                    labels,
                    ..Default::default()
                },
                format: Format::Compact(LayerConfig::compact()),
                target: WriterTarget::Stdout,
                show_path: false,
                show_spans: false,
                ..Default::default()
            },
            || emit_demo(label),
        );
    }

    section("PATH & SPANS");
    for (show_path, show_spans, desc) in [
        (true, true, "path=on, spans=on"),
        (false, false, "path=off, spans=off"),
    ] {
        log!(sub, desc);
        run_with(
            &Writer {
                show_path,
                show_spans,
                ..Default::default()
            },
            || emit_demo(desc),
        );
    }

    section("TIME FORMAT");
    for (tf, desc) in [
        (None, "default: %H:%M:%S"),
        (
            Some("%Y-%m-%d %H:%M:%S%.3f".into()),
            "custom: %Y-%m-%d %H:%M:%S%.3f",
        ),
    ] {
        log!(sub, desc);
        run_with(
            &Writer {
                time_format: tf,
                show_path: false,
                show_spans: false,
                ..Default::default()
            },
            || emit_demo(desc),
        );
    }

    section("TARGETS");
    for (target, desc) in [
        (WriterTarget::Stdout, "stdout"),
        (WriterTarget::Stderr, "stderr"),
    ] {
        log!(sub, desc);
        if matches!(target, WriterTarget::Stderr) {
            eprintln!("    (output to stderr)");
        }
        run_with(
            &Writer {
                target,
                show_path: false,
                show_spans: false,
                ..Default::default()
            },
            || emit_demo(desc),
        );
    }

    section("SPANS");
    run_with(
        &Writer {
            show_spans: true,
            ..Default::default()
        },
        emit_spans,
    );

    section("INFRA");

    log!(sub, "Level → directive");
    for l in [
        Level::Error,
        Level::Warn,
        Level::Info,
        Level::Debug,
        Level::Trace,
        Level::Off,
    ] {
        log!(info, &format!("{l:?} → \"{}\"", l.as_directive()));
    }
    log!(
        info,
        &format!(
            "Filter::from_directive(\"info,my_crate=debug\") → \"{}\"",
            Filter::from_directive("info,my_crate=debug").as_directive()
        )
    );

    log!(sub, "build_layer");
    for (desc, w) in [
        ("default", Writer::default()),
        (
            "json+stderr+no-ansi",
            Writer {
                format: Format::Json(LayerConfig::json()),
                ansi: false,
                target: WriterTarget::Stderr,
                ..Default::default()
            },
        ),
    ] {
        drop(build_layer(&w));
        log!(success, &format!("build_layer({desc})"));
    }

    section("FILE");

    log!(sub, "rotate_log_file");
    let dir = std::path::Path::new("data/logs/rotation");
    drop(std::fs::create_dir_all(dir));
    let path = dir.join("test.log");
    log!(info, &format!("target: {}", path.display()));
    std::fs::write(&path, b"old\n").ok();
    for (mode, name) in [
        (Rotation::Rename, "Rename"),
        (Rotation::Compress, "Compress"),
    ] {
        match rotate_log_file(&path, mode) {
            Ok(()) => log!(success, &format!("rotate({name})")),
            Err(e) => log!(fail, &format!("rotate({name}): {e}")),
        }
        std::fs::write(&path, b"fresh\n").ok();
    }
    if let Ok(entries) = std::fs::read_dir(dir) {
        log!(info, "disk:");
        for e in entries.flatten() {
            log!(pad, e.file_name().to_string_lossy());
        }
    }

    section("RELOAD via init");
    log!(sub, "init + runtime reload");
    let dir = std::path::Path::new("data/logs/full");
    drop(std::fs::create_dir_all(dir));
    let config = Config::builder()
        .level(Level::Debug)
        .with_writer(Writer {
            format: Format::Compact(LayerConfig::compact()),
            show_path: false,
            show_spans: false,
            target: WriterTarget::Stdout,
            ..Default::default()
        })
        .with_writer(Writer {
            format: Format::Json(LayerConfig::json()),
            target: WriterTarget::File {
                path: dir.join("app.log"),
                rotation: Rotation::default(),
            },
            ..Default::default()
        })
        .build();
    match init(config) {
        Ok(mut g) => {
            log!(success, "init");
            if let Some(p) = g.log_path() {
                log!(info, &format!("file → {}", p.display()));
            }
            tracing::info!(init = true, "console + file");

            g.set_level(Level::Warn).unwrap();
            log!(info, "→ set_level(Warn)");
            tracing::info!("info suppressed");
            tracing::warn!("warn passes");

            g.set_target_level("demo", Level::Trace).unwrap();
            log!(info, "→ set_target_level(demo, Trace)");
            tracing::trace!(target: "demo", "demo trace passes");

            g.with_style(|s| s.theme = Theme::monokai());
            log!(info, "→ style switch (monokai)");
            tracing::warn!("monokai");

            drop(g);
        }
        Err(e) => log!(fail, &format!("init: {e}")),
    }
}
