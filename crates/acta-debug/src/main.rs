#![allow(clippy::print_stdout, clippy::print_stderr, clippy::unwrap_used)]
use std::sync::LazyLock;

use acta::{
    ColorDepth, Config, Format, Formatter, Icons, JsonOptions, LevelLabels, PrettyOptions, Style,
    Theme, Writer, WriterTarget, init,
};
use smallvec::{SmallVec, smallvec};
use tracing::Level;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::Registry;
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::fmt::writer::BoxMakeWriter;
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

fn run_with(format: Format, target: WriterTarget, color_depth: ColorDepth, f: impl FnOnce()) {
    let base = tracing_subscriber::fmt::Layer::default()
        .with_thread_ids(false)
        .with_thread_names(false)
        .with_span_events(FmtSpan::NONE)
        .with_writer(match target {
            WriterTarget::Stderr => BoxMakeWriter::new(std::io::stderr),
            _ => BoxMakeWriter::new(std::io::stdout),
        })
        .with_ansi(color_depth != ColorDepth::NoColor);

    let layer: Box<dyn tracing_subscriber::Layer<Registry> + Send + Sync> = match format {
        Format::Pretty(cfg) => base
            .pretty()
            .with_target(cfg.target)
            .with_file(cfg.file)
            .with_line_number(cfg.line_number)
            .boxed(),
        Format::Compact(formatter) => base
            .event_format(formatter.with_color_depth(color_depth))
            .boxed(),
        Format::Json(cfg) => base
            .json()
            .with_target(cfg.target)
            .with_file(cfg.file)
            .with_line_number(cfg.line_number)
            .with_current_span(cfg.current_span)
            .with_span_list(cfg.span_list)
            .flatten_event(cfg.flatten_event)
            .boxed(),
        _ => base.boxed(),
    };

    tracing::subscriber::with_default(tracing_subscriber::registry().with(layer), f);
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

static ICONS: LazyLock<SmallVec<[(&str, Icons); 3]>> = LazyLock::new(|| {
    smallvec![
        ("unicode", Icons::UNICODE),
        (
            "no-icons",
            Icons::custom("no-icons", "", "", "", "", "", "", "")
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
        (
            "compact",
            Format::Compact(
                Formatter::new()
                    .with_show_path(false)
                    .with_show_spans(false),
            ),
        ),
        ("pretty", Format::Pretty(PrettyOptions::default())),
        ("json", Format::Json(JsonOptions::default())),
    ];
    for (fmt_name, format) in formats {
        for (icon_name, icons) in &*ICONS {
            let style = Style {
                icons: *icons,
                ..Default::default()
            };
            let format = match format {
                Format::Compact(f) => Format::Compact(f.clone().with_style(style)),
                other => other.clone(),
            };
            let color_depth = if matches!(format, Format::Json(_)) {
                ColorDepth::NoColor
            } else {
                ColorDepth::TrueColor
            };
            log!(sub, &format!("{fmt_name} + {icon_name}"));
            run_with(format, WriterTarget::Stdout, color_depth, emit_all_levels);
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
        for (icon_name, icons) in &*ICONS {
            println!("  [{name} / {icon_name}]");
            run_with(
                Format::Compact(
                    Formatter::new()
                        .with_style(Style {
                            theme: *theme,
                            icons: *icons,
                            ..Default::default()
                        })
                        .with_show_path(false)
                        .with_show_spans(false),
                ),
                WriterTarget::Stdout,
                ColorDepth::TrueColor,
                || emit_demo(name),
            );
        }
    }

    section("ALL LEVELS");
    for (icon_name, icons) in &*ICONS {
        for depth in [
            ColorDepth::TrueColor,
            ColorDepth::Ansi256,
            ColorDepth::Ansi16,
            ColorDepth::NoColor,
        ] {
            log!(sub, &format!("{icon_name} / {depth:?}"));
            run_with(
                Format::Compact(
                    Formatter::new()
                        .with_style(Style {
                            icons: *icons,
                            theme: Theme::one_dark(),
                            ..Default::default()
                        })
                        .with_show_path(false)
                        .with_show_spans(false),
                ),
                WriterTarget::Stdout,
                depth,
                emit_all_levels,
            );
        }
    }

    section("LABELS");
    for (label, labels) in [
        ("short", LevelLabels::SHORT),
        ("medium", LevelLabels::MEDIUM),
        ("long", LevelLabels::LONG),
    ] {
        log!(sub, label);
        run_with(
            Format::Compact(
                Formatter::new()
                    .with_style(Style {
                        labels,
                        ..Default::default()
                    })
                    .with_show_path(false)
                    .with_show_spans(false),
            ),
            WriterTarget::Stdout,
            ColorDepth::TrueColor,
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
            Format::Compact(
                Formatter::new()
                    .with_show_path(show_path)
                    .with_show_spans(show_spans),
            ),
            WriterTarget::Stdout,
            ColorDepth::TrueColor,
            || emit_demo(desc),
        );
    }

    section("TIME FORMAT");
    for (tf, desc) in [
        (None, "default: %H:%M:%S"),
        (
            Some("%Y-%m-%d %H:%M:%S%.3f"),
            "custom: %Y-%m-%d %H:%M:%S%.3f",
        ),
    ] {
        log!(sub, desc);
        let mut formatter = Formatter::new()
            .with_show_path(false)
            .with_show_spans(false);
        if let Some(tf) = tf {
            formatter = formatter.with_time_format(tf);
        }
        run_with(
            Format::Compact(formatter),
            WriterTarget::Stdout,
            ColorDepth::TrueColor,
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
            Format::Compact(
                Formatter::new()
                    .with_show_path(false)
                    .with_show_spans(false),
            ),
            target,
            ColorDepth::TrueColor,
            || emit_demo(desc),
        );
    }

    section("SPANS");
    for (icon_name, icons) in &*ICONS {
        log!(sub, icon_name);
        run_with(
            Format::Compact(Formatter::new().with_style(Style {
                icons: *icons,
                ..Default::default()
            })),
            WriterTarget::Stdout,
            ColorDepth::TrueColor,
            || {
                tracing::info!("no span");
                let _a = tracing::info_span!("layer1").entered();
                tracing::info!("span [layer1]");
                let _b = tracing::info_span!("layer2", depth = 2).entered();
                tracing::warn!("span [layer1 > layer2{{depth=2}}]");
                let _c = tracing::debug_span!("layer3").entered();
                tracing::error!("span [layer1 > layer2 > layer3]");
            },
        );
    }

    section("INFRA");

    log!(sub, "Level → directive");
    for l in [
        Level::ERROR,
        Level::WARN,
        Level::INFO,
        Level::DEBUG,
        Level::TRACE,
    ] {
        log!(info, &format!("{l} → \"{}\"", l.as_str().to_lowercase()));
    }
    log!(info, "Off → \"off\"");
    log!(info, "Filter → EnvFilter::new(\"info,my_crate=debug\")");

    section("RELOAD via init");
    log!(sub, "init + runtime reload");
    let dir = std::path::Path::new("data/logs/full");
    drop(std::fs::create_dir_all(dir));
    match init(
        Config::builder()
            .with_filter(EnvFilter::new("debug"))
            .with_writers([
                Writer::stdout().with_format(Format::Compact(
                    Formatter::new()
                        .with_show_path(false)
                        .with_show_spans(false),
                )),
                Writer::file(dir.join("app.log")).with_format(Format::Json(JsonOptions::default())),
            ])
            .build(),
    ) {
        Ok(g) => {
            log!(success, "init");
            tracing::info!(init = true, "console + file");

            g.set_filter(EnvFilter::new("warn,demo=trace")).unwrap();
            log!(info, "→ set_filter(\"warn,demo=trace\")");
            tracing::info!("info suppressed");
            tracing::warn!("warn passes");
            tracing::trace!(target: "demo", "demo trace passes");

            drop(g);
        }
        Err(e) => log!(fail, &format!("init: {e}")),
    }
}
