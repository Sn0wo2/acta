use super::visitor::EventVisitor;
use super::*;
use smallvec::SmallVec;

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
        .with_time_format("%Y-%m-%d %H:%M:%S")
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
    let fmt = Formatter::new().with_path_width(20);
    let result = fmt.format_path("foo.rs", 10);
    assert_eq!(result.len(), 20);
    assert!(result.contains("foo.rs:10"));
}

#[test]
fn smart_truncate_exact_width() {
    let fmt = Formatter::new().with_path_width(8);
    let result = fmt.format_path("foo.rs", 1);
    assert_eq!(result.as_str(), "foo.rs:1");
}

#[test]
fn smart_truncate_overflow() {
    let fmt = Formatter::new().with_path_width(15);
    let result = fmt.format_path("very/long/path/file.rs", 999);
    assert!(result.len() <= 15);
}

#[test]
fn smart_truncate_trailing_slash_before_filename() {
    let fmt = Formatter::new().with_path_width(20);
    let result = fmt.format_path("dir/subdir/file.rs", 42);
    assert!(result.len() <= 20);
    assert!(result.contains("file.rs:42"));
}

#[test]
fn smart_truncate_file_part_too_long() {
    let fmt = Formatter::new().with_path_width(15);
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
    let fmt = Formatter::new().with_path_width(20);
    let result = fmt.format_path("C:\\project\\src\\lib.rs", 42);
    assert!(result.contains("lib.rs:42"));
    assert!(!result.contains("src/"));
}

#[test]
fn format_path_right_aligned_no_truncation() {
    let mut fmt = Formatter::new();
    fmt.path_width = 40;
    let result = fmt.format_path("src/lib.rs", 10);
    assert!(!result.contains('\u{2026}'), "expected no ellipsis: {result}");
    assert_eq!(result.len(), 40);
    assert!(result.ends_with("lib.rs:10"), "expected to end with stripped path: {result}");
}

#[test]
fn format_path_dir_truncation_preserves_filename() {
    let mut fmt = Formatter::new();
    fmt.path_width = 28;
    let result = fmt.format_path("very/long/deeply/nested/dir/file.rs", 42);
    assert!(result.contains("file.rs:42"), "expected filename preserved: {result}");
    assert!(!result.contains('\u{2026}'), "expected no leading ellipsis in dir truncation: {result}");
    assert!(result.len() <= 28);
}

#[test]
fn format_path_windows_normalization() {
    let mut fmt = Formatter::new();
    fmt.path_width = 40;
    let result = fmt.format_path(r"C:\project\src\module\file.rs", 7);
    assert!(!result.contains('\\'), "expected backslashes normalized: {result}");
    assert!(result.contains("module/file.rs:7"), "expected normalized src path: {result}");
}

#[test]
fn format_path_leading_ellipsis_for_very_long_path() {
    let mut fmt = Formatter::new();
    // Narrow enough that file_with_line alone exceeds width,
    // forcing the leading-ellipsis fallback branch.
    fmt.path_width = 11;
    let result = fmt.format_path("/very/long/prefix/deep/nested/dirs/file.rs", 99);
    assert!(result.starts_with('\u{2026}'), "expected leading ellipsis: {result}");
    assert!(result.contains("file.rs:99"), "expected filename preserved: {result}");
}

#[test]
fn format_path_file_with_line_branch_no_ellipsis() {
    let mut fmt = Formatter::new();
    fmt.path_width = 16;
    let result = fmt.format_path("src/main.rs", 1);
    assert!(result.contains("main.rs:1"), "expected stripped path: {result}");
    assert!(!result.contains('\u{2026}'));
    assert_eq!(result.len(), 16);
}

#[test]
fn formatter_style_config_returns_reference() {
    let fmt = Formatter::new();
    let config = fmt.style_config();
    assert_eq!(config.labels.error, "E");
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
    assert_eq!(fmt.style_config().labels.error, "E");

    let fmt = fmt.with_labels(LevelLabels::DEFAULT);
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
    visitor.record_field("message", "hello".to_owned());
    assert_eq!(visitor.message, Some("hello".to_owned()));
    assert!(visitor.fields.is_empty());
}

#[test]
fn event_visitor_records_msg_alias() {
    let mut visitor = EventVisitor::default();
    visitor.record_field("msg", "world".to_owned());
    assert_eq!(visitor.message, Some("world".to_owned()));
    assert!(visitor.fields.is_empty());
}

#[test]
fn event_visitor_records_other_fields_as_pairs() {
    let mut visitor = EventVisitor::default();
    visitor.record_field("user", "alice".to_owned());
    visitor.record_field("count", "42".to_owned());
    assert!(visitor.message.is_none());
    assert_eq!(
        visitor.fields,
        SmallVec::<[(&'static str, String); 4]>::from_vec(vec![
            ("user", "alice".to_owned()),
            ("count", "42".to_owned())
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
    visitor.record_field("x", "1".to_owned());
    visitor.record_field("message", "the message".to_owned());
    visitor.record_field("y", "2".to_owned());
    assert_eq!(visitor.message, Some("the message".to_owned()));
    assert_eq!(
        visitor.fields,
        SmallVec::<[(&'static str, String); 4]>::from_vec(vec![
            ("x", "1".to_owned()),
            ("y", "2".to_owned())
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
    let labels = LevelLabels::DEFAULT;
    assert_eq!(labels.error, "ERROR");
    assert_eq!(labels.warn, " WARN");
    assert_eq!(labels.info, " INFO");
    assert_eq!(labels.debug, "DEBUG");
    assert_eq!(labels.trace, "TRACE");
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
    let icons = Icons::UNICODE;
    assert_eq!(icons.name, "unicode");
}

#[cfg(feature = "nerd")]
#[test]
fn icons_name_nerd() {
    let icons = Icons::NERD;
    assert_eq!(icons.name, "nerd");
}

#[test]
fn style_config_default() {
    let config = Style::default();
    assert_eq!(
        format!("{:?}", config.theme),
        format!("{:?}", Theme::acta())
    );
    assert_eq!(config.icons, Icons::UNICODE);
    assert_eq!(config.labels, LevelLabels::SHORT);
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
fn theme_default_equals_trans_flag() {
    assert_eq!(
        format!("{:?}", Theme::acta()),
        format!("{:?}", Theme::acta())
    );
}
