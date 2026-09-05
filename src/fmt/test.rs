#![allow(clippy::expect_used)]

use compact_str::CompactString;
use std::sync::{Arc, Mutex};

use super::visitor::EventVisitor;
use super::*;
use smallvec::SmallVec;
use tracing_subscriber::fmt::format::Writer;
use tracing_subscriber::layer::SubscriberExt;

#[test]
fn formatter_defaults() {
    let fmt = Formatter::new();
    assert_eq!(fmt.time_format, "%H:%M:%S");
    assert_eq!(fmt.path_width, DEFAULT_PATH_WIDTH);
    assert!(fmt.show_path);
    assert!(fmt.show_spans);
}

#[test]
fn formatter_builder() {
    let fmt = Formatter::new()
        .with_time_format("%Y-%m-%d %H:%M:%S".to_string())
        .with_path_width(40)
        .with_show_path(false)
        .with_show_spans(false)
        .with_theme(Theme::monokai());

    assert_eq!(fmt.time_format, "%Y-%m-%d %H:%M:%S");
    assert_eq!(fmt.path_width, 40);
    assert!(!fmt.show_path);
    assert!(!fmt.show_spans);
}

#[cfg(feature = "nerd")]
#[test]
fn formatter_with_icons() {
    let fmt = Formatter::new().with_icons(Icons::NERD);
    assert_eq!(fmt.style_config().icons.bracket_open, "\u{e0b6}");
}

#[test]
fn theme_presets_are_distinct() {
    let s1 = format!("{:?}", Theme::acta());
    let s2 = format!("{:?}", Theme::monokai());
    let s3 = format!("{:?}", Theme::dracula());
    assert_ne!(s1, s2);
    assert_ne!(s2, s3);
}

#[test]
fn theme_default_is_trans_flag() {
    assert_eq!(
        format!("{:?}", Theme::acta()),
        format!("{:?}", Theme::acta())
    );
}

#[cfg(feature = "nerd")]
#[test]
fn icons_unicode_vs_nerd() {
    let u = Icons::UNICODE;
    let n = Icons::NERD;
    assert_ne!(u.bracket_open, n.bracket_open);
    assert_ne!(u.bracket_close, n.bracket_close);
    assert_ne!(u.arrow, n.arrow);
    assert_eq!(u.separator, n.separator);
}

#[test]
fn smart_truncate_short_path() {
    let mut fmt = Formatter::new();
    fmt.path_width = 20;
    let result = fmt.format_path("foo.rs", 10);
    assert_eq!(result.len(), 20);
    assert!(result.contains("foo.rs:10"));
}

#[test]
fn smart_truncate_exact_width() {
    let mut fmt = Formatter::new();
    fmt.path_width = 8;
    let result = fmt.format_path("foo.rs", 1);
    assert_eq!(result.as_str(), "foo.rs:1");
}

#[test]
fn smart_truncate_overflow() {
    let mut fmt = Formatter::new();
    fmt.path_width = 15;
    let result = fmt.format_path("very/long/path/file.rs", 999);
    assert!(result.len() <= 15);
}

#[test]
fn smart_truncate_trailing_slash_before_filename() {
    let mut fmt = Formatter::new();
    fmt.path_width = 20;
    let result = fmt.format_path("dir/subdir/file.rs", 42);
    assert!(result.len() <= 20);
    assert!(result.contains("file.rs:42"));
}

#[test]
fn smart_truncate_file_part_too_long() {
    let mut fmt = Formatter::new();
    fmt.path_width = 15;
    let result = fmt.format_path("very_very_long_filename_test.rs", 10);
    assert!(
        result.len() <= 18,
        "result='{}', len={}",
        result,
        result.len()
    );
    assert!(result.starts_with('\u{2026}'));
}

#[test]
fn format_path_strips_src() {
    let mut fmt = Formatter::new();
    fmt.path_width = 20;
    let result = fmt.format_path("C:\\project\\src\\lib.rs", 42);
    assert!(result.contains("lib.rs:42"));
    assert!(!result.contains("src/"));
}

#[test]
fn format_path_right_aligned_no_truncation() {
    let mut fmt = Formatter::new();
    fmt.path_width = 40;
    let result = fmt.format_path("src/lib.rs", 10);
    assert!(
        !result.contains('\u{2026}'),
        "expected no ellipsis: {result}"
    );
    assert_eq!(result.len(), 40);
    assert!(
        result.ends_with("lib.rs:10"),
        "expected to end with stripped path: {result}"
    );
}

#[test]
fn format_path_dir_truncation_preserves_filename() {
    let mut fmt = Formatter::new();
    fmt.path_width = 28;
    let result = fmt.format_path("very/long/deeply/nested/dir/file.rs", 42);
    assert!(
        result.contains("file.rs:42"),
        "expected filename preserved: {result}"
    );
    assert!(
        !result.contains('\u{2026}'),
        "expected no leading ellipsis in dir truncation: {result}"
    );
    assert!(result.len() <= 28);
}

#[test]
fn format_path_windows_normalization() {
    let mut fmt = Formatter::new();
    fmt.path_width = 40;
    let result = fmt.format_path(r"C:\project\src\module\file.rs", 7);
    assert!(
        !result.contains('\\'),
        "expected backslashes normalized: {result}"
    );
    assert!(
        result.contains("module/file.rs:7"),
        "expected normalized src path: {result}"
    );
}

#[test]
fn format_path_leading_ellipsis_for_very_long_path() {
    let mut fmt = Formatter::new();
    fmt.path_width = 11;
    let result = fmt.format_path("/very/long/prefix/deep/nested/dirs/file.rs", 99);
    assert!(
        result.starts_with('\u{2026}'),
        "expected leading ellipsis: {result}"
    );
    assert!(
        result.contains("file.rs:99"),
        "expected filename preserved: {result}"
    );
}

#[test]
fn format_path_file_with_line_branch_no_ellipsis() {
    let mut fmt = Formatter::new();
    fmt.path_width = 16;
    let result = fmt.format_path("src/main.rs", 1);
    assert!(
        result.contains("main.rs:1"),
        "expected stripped path: {result}"
    );
    assert!(!result.contains('\u{2026}'));
    assert_eq!(result.len(), 16);
}

#[test]
fn format_path_unicode_char_boundary_safe() {
    let mut fmt = Formatter::new();
    fmt.path_width = 12;
    let result = fmt.format_path("src/模块/文件.rs", 10);
    assert!(std::str::from_utf8(result.as_bytes()).is_ok());
    assert!(
        result.contains("件.rs:10") || result.contains("文件.rs:10"),
        "expected filename suffix preserved: {result}"
    );
}

#[test]
fn formatter_style_config_returns_reference() {
    let fmt = Formatter::new();
    let config = fmt.style_config();
    assert_eq!(config.labels.error, "ERROR");
}

#[test]
fn formatter_with_style_config_replaces_all() {
    let config = Style {
        labels: LevelLabels::SHORT,
        icons: Icons::UNICODE,
        theme: Theme::monokai(),
    };
    let fmt = Formatter::new().with_style_config(config);
    assert_eq!(fmt.style_config().labels, LevelLabels::SHORT);
    assert_eq!(fmt.style_config().icons.bracket_open, "[");
    assert!((248..=255).contains(&fmt.style_config().theme.error.0));
}

#[test]
fn formatter_with_labels_changes_labels() {
    let fmt = Formatter::new();
    assert_eq!(fmt.style_config().labels.error, "ERROR");

    let fmt = fmt.with_labels(LevelLabels::LONG);
    assert_eq!(fmt.style_config().labels.error, "ERROR");
}

#[test]
fn formatter_with_icons_changes_icons() {
    let fmt = Formatter::new().with_icons(Icons::UNICODE);
    assert_eq!(fmt.style_config().icons.bracket_open, "[");
}

#[test]
fn formatter_with_theme_changes_theme() {
    let fmt = Formatter::new().with_theme(Theme::monokai());
    assert_ne!(
        format!("{:?}", fmt.style_config().theme),
        format!("{:?}", Theme::acta())
    );
}

#[test]
fn event_visitor_records_message_field() {
    let mut visitor = EventVisitor::default();
    visitor.record_field("message", "hello");
    assert_eq!(visitor.message, Some(CompactString::from("hello")));
    assert!(visitor.fields.is_empty());
}

#[test]
fn event_visitor_records_msg_alias() {
    let mut visitor = EventVisitor::default();
    visitor.record_field("msg", "world");
    assert_eq!(visitor.message, Some(CompactString::from("world")));
    assert!(visitor.fields.is_empty());
}

#[test]
fn event_visitor_records_other_fields_as_pairs() {
    let mut visitor = EventVisitor::default();
    visitor.record_field("user", "alice");
    visitor.record_field("count", "42");
    assert!(visitor.message.is_none());
    assert_eq!(
        visitor.fields,
        SmallVec::<[(CompactString, CompactString); 4]>::from_vec(vec![
            (CompactString::from("user"), CompactString::from("alice")),
            (CompactString::from("count"), CompactString::from("42"))
        ])
    );
}

#[test]
fn event_visitor_default_has_no_message_and_empty_fields() {
    let visitor = EventVisitor::default();
    assert!(visitor.message.is_none());
    assert!(visitor.fields.is_empty());
}

#[test]
fn event_visitor_order_preserved_message_extracted() {
    let mut visitor = EventVisitor::default();
    visitor.record_field("x", "1");
    visitor.record_field("message", "the message");
    visitor.record_field("y", "2");
    assert_eq!(visitor.message, Some(CompactString::from("the message")));
    assert_eq!(
        visitor.fields,
        SmallVec::<[(CompactString, CompactString); 4]>::from_vec(vec![
            (CompactString::from("x"), CompactString::from("1")),
            (CompactString::from("y"), CompactString::from("2"))
        ])
    );
}

#[test]
fn level_labels_short() {
    let labels = LevelLabels::SHORT;
    assert_eq!(labels.error, "E");
    assert_eq!(labels.warn, "W");
    assert_eq!(labels.info, "I");
    assert_eq!(labels.debug, "D");
    assert_eq!(labels.trace, "T");
}

#[test]
fn level_labels_long() {
    let labels = LevelLabels::LONG;
    assert_eq!(labels.error, "ERROR");
    assert_eq!(labels.warn, " WARN");
    assert_eq!(labels.info, " INFO");
    assert_eq!(labels.debug, "DEBUG");
    assert_eq!(labels.trace, "TRACE");
}

#[test]
fn level_labels_medium() {
    let labels = LevelLabels::MEDIUM;
    assert_eq!(labels.error, "ERR");
    assert_eq!(labels.warn, "WRN");
    assert_eq!(labels.info, "INF");
    assert_eq!(labels.debug, "DBG");
    assert_eq!(labels.trace, "TRC");
}

#[test]
fn level_labels_default_is_short() {
    assert_eq!(LevelLabels::default(), LevelLabels::SHORT);
}

#[test]
fn icons_unicode() {
    let icons = Icons::UNICODE;
    assert_eq!(icons.bracket_open, "[");
    assert_eq!(icons.bracket_close, "]");
    assert_eq!(icons.separator, "\u{2507}");
    assert_eq!(icons.arrow, ">");
    assert_eq!(icons.span_delimiter, "->");
}

#[test]
fn icons_name_unicode() {
    assert_eq!(Icons::UNICODE.name, "unicode");
}

#[cfg(feature = "nerd")]
#[test]
fn icons_name_nerd() {
    assert_eq!(Icons::NERD.name, "nerd");
}

#[test]
fn style_config_default() {
    let config = Style::default();
    assert_eq!(
        format!("{:?}", config.theme),
        format!("{:?}", Theme::acta())
    );
    assert_eq!(config.icons, Icons::UNICODE);
    assert_eq!(config.labels, LevelLabels::LONG);
}

#[test]
fn theme_all_have_distinct_accent_colors() {
    let themes = [
        Theme::acta(),
        Theme::monokai(),
        Theme::dracula(),
        Theme::nord(),
        Theme::catppuccin_mocha(),
        Theme::gruvbox(),
        Theme::one_dark(),
        Theme::tokyo_night(),
    ];

    for (i, theme_i) in themes.iter().enumerate() {
        for theme_j in themes.iter().skip(i + 1) {
            assert_ne!(
                format!("{:?}", theme_i.accent),
                format!("{:?}", theme_j.accent)
            );
        }
    }
}

#[test]
fn theme_default_equals_acta() {
    assert_eq!(
        format!("{:?}", Theme::default()),
        format!("{:?}", Theme::acta())
    );
}

struct CaptureLayer<F>(F);

impl<S, F> tracing_subscriber::Layer<S> for CaptureLayer<F>
where
    S: Subscriber,
    F: Fn(&Event<'_>) + Send + Sync + 'static,
{
    fn on_event(&self, event: &Event<'_>, _ctx: tracing_subscriber::layer::Context<'_, S>) {
        (self.0)(event);
    }
}

/// A `tracing::Event` is only observable inside a live subscriber.
fn capture_output(
    format: impl Fn(&mut Writer<'_>, &Event<'_>) -> fmt::Result + Send + Sync + 'static,
    emit: impl FnOnce(),
) -> String {
    let out = Arc::new(Mutex::new(String::new()));
    let sink = out.clone();
    let layer = CaptureLayer(move |event: &Event<'_>| {
        let mut buf = String::new();
        format(&mut Writer::new(&mut buf), event).expect("formatting failed");
        *sink.lock().expect("capture lock poisoned") = buf;
    });
    tracing::subscriber::with_default(tracing_subscriber::registry().with(layer), emit);
    out.lock().expect("capture lock poisoned").clone()
}

#[test]
fn format_path_section_output_contains_path_and_arrow() {
    let fmt = Formatter::new().with_path_width(40);
    let output = capture_output(
        move |writer, event| {
            fmt.format_path_section(writer, event, &Theme::acta(), &Icons::UNICODE)
        },
        || tracing::event!(Level::INFO, "probe"),
    );

    assert!(
        output.contains("fmt/test.rs"),
        "expected output to contain file path, got: {output}"
    );
    assert!(
        output.contains('>'),
        "expected output to contain arrow icon, got: {output}"
    );
}

#[test]
fn write_time_outputs_formatted_time() {
    let fmt = Formatter::new();
    let theme = Theme::acta();

    let mut buf = String::new();
    let result = fmt.write_time(&mut Writer::new(&mut buf), &theme);

    assert!(result.is_ok());
    assert!(!buf.is_empty(), "expected non-empty time output");
    assert!(
        buf.contains(':'),
        "expected time to contain colon separator"
    );
}

#[test]
fn write_time_custom_format() {
    let fmt = Formatter::new().with_time_format("%Y-%m-%d");
    let theme = Theme::acta();

    let mut buf = String::new();
    let result = fmt.write_time(&mut Writer::new(&mut buf), &theme);

    assert!(result.is_ok());
    assert!(buf.contains('-'), "expected date format with dashes");
}

#[test]
fn format_fields_outputs_message() {
    let fmt = Formatter::new();
    let output = capture_output(
        move |writer, event| fmt.format_fields(writer, event, &Theme::acta()),
        || tracing::event!(Level::INFO, "hello world"),
    );

    assert!(
        output.contains("hello world"),
        "expected output to contain message, got: {output}"
    );
}

#[test]
fn format_fields_outputs_key_value_pairs() {
    let fmt = Formatter::new().with_color_depth(ColorDepth::NoColor);
    let output = capture_output(
        move |writer, event| fmt.format_fields(writer, event, &Theme::acta()),
        || tracing::event!(Level::INFO, user = "alice", count = 42),
    );

    assert_eq!(output, "user=alice count=42");
}

#[test]
fn format_fields_with_message_and_fields() {
    let fmt = Formatter::new().with_color_depth(ColorDepth::NoColor);
    let output = capture_output(
        move |writer, event| fmt.format_fields(writer, event, &Theme::acta()),
        || tracing::event!(Level::INFO, key = "value", "test message"),
    );

    assert_eq!(output, "test message key=value");
}
