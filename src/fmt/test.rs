#![allow(clippy::expect_used)]

use compact_str::CompactString;

use super::visitor::EventVisitor;
use super::*;
use crate::config::LevelLabels;
use smallvec::SmallVec;

#[test]
fn formatter_defaults() {
    let fmt = Formatter::new();
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

    assert_eq!(fmt.path_width, 40);
    assert!(!fmt.show_path);
    assert!(!fmt.show_spans);
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
fn formatter_with_theme_changes_theme() {
    let fmt = Formatter::new().with_theme(Theme::monokai());
    assert_ne!(
        format!("{:?}", fmt.style.theme),
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
        SmallVec::<[(&'static str, CompactString); 4]>::from_vec(vec![
            ("user", CompactString::from("alice")),
            ("count", CompactString::from("42"))
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
        SmallVec::<[(&'static str, CompactString); 4]>::from_vec(vec![
            ("x", CompactString::from("1")),
            ("y", CompactString::from("2"))
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

#[test]
fn time_format_default_renders_time() {
    let fmt = Formatter::new();
    let rendered = Local::now()
        .format_with_items(fmt.time_items.iter())
        .to_string();

    assert!(!rendered.is_empty(), "expected non-empty time output");
    assert!(
        rendered.contains(':'),
        "expected time to contain colon separator"
    );
}

#[test]
fn time_format_custom_renders_date() {
    let fmt = Formatter::new().with_time_format("%Y-%m-%d");
    let rendered = Local::now()
        .format_with_items(fmt.time_items.iter())
        .to_string();

    assert_eq!(rendered.len(), 10);
    assert_eq!(rendered.matches('-').count(), 2);
}

#[test]
fn time_format_invalid_specifier_yields_error_item() {
    let fmt = Formatter::new().with_time_format("%Q");
    assert!(
        fmt.time_items
            .iter()
            .any(|item| matches!(item, Item::Error))
    );
}
