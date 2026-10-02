use crate::config::ColorDepth;
use crate::config::Style;
use crate::config::Theme;
use chrono::Local;
use chrono::format::Item;
use compact_str::format_compact;
use std::fmt;
use std::path::{Path, PathBuf};
use tracing::{Event, Level, Subscriber};
use tracing_subscriber::fmt::FormattedFields;
use tracing_subscriber::fmt::format::Writer;
use tracing_subscriber::fmt::{FmtContext, FormatEvent};
use tracing_subscriber::registry::LookupSpan;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

mod visitor;
use crate::utils;
use visitor::EventVisitor;

// build.rs will generate the path_width file to output dir
const DEFAULT_PATH_WIDTH: usize = include!(concat!(env!("OUT_DIR"), "/path_width"));

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct Formatter {
    pub(crate) style: Style,
    pub(crate) color_depth: ColorDepth,
    pub(crate) time_items: Vec<Item<'static>>,
    pub(crate) path_width: usize,
    pub(crate) show_path: bool,
    pub(crate) show_spans: bool,
}

impl Default for Formatter {
    fn default() -> Self {
        Self::new()
    }
}

impl Formatter {
    #[must_use]
    pub fn new() -> Self {
        Self {
            style: Style::default(),
            color_depth: ColorDepth::TrueColor,
            time_items: utils::time::parse_time_items("%H:%M:%S"),
            path_width: DEFAULT_PATH_WIDTH,
            show_path: true,
            show_spans: true,
        }
    }

    #[must_use]
    pub const fn with_style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    #[must_use]
    pub const fn with_theme(mut self, theme: Theme) -> Self {
        self.style.theme = theme;
        self
    }

    #[must_use]
    pub const fn with_color_depth(mut self, depth: ColorDepth) -> Self {
        self.color_depth = depth;
        self
    }

    #[must_use]
    pub const fn with_path_width(mut self, width: usize) -> Self {
        self.path_width = width;
        self
    }

    /// Timestamps use the local system timezone.
    #[must_use]
    pub fn with_time_format(mut self, fmt: impl Into<String>) -> Self {
        self.time_items = utils::time::parse_time_items(&fmt.into());
        self
    }

    #[must_use]
    pub const fn with_show_path(mut self, show: bool) -> Self {
        self.show_path = show;
        self
    }

    #[must_use]
    pub const fn with_show_spans(mut self, show: bool) -> Self {
        self.show_spans = show;
        self
    }
}

impl<S, N> FormatEvent<S, N> for Formatter
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'a> tracing_subscriber::fmt::FormatFields<'a> + 'static,
{
    fn format_event(
        &self,
        ctx: &FmtContext<'_, S, N>,
        mut writer: Writer<'_>,
        event: &Event<'_>,
    ) -> fmt::Result {
        let config = &self.style;
        let theme = &config.theme;
        let icons = &config.icons;

        let (color, level_label) = match *event.metadata().level() {
            Level::ERROR => (theme.error, config.labels.error),
            Level::WARN => (theme.warn, config.labels.warn),
            Level::INFO => (theme.info, config.labels.info),
            Level::DEBUG => (theme.debug, config.labels.debug),
            Level::TRACE => (theme.trace, config.labels.trace),
        };
        let accent =
            owo_colors::Style::new().color(utils::color::themed(theme.accent, self.color_depth));
        let text =
            owo_colors::Style::new().color(utils::color::themed(theme.text, self.color_depth));
        let secondary =
            owo_colors::Style::new().color(utils::color::themed(theme.secondary, self.color_depth));

        write!(writer, "{}", accent.style(icons.time_bracket_open))?;
        write!(
            writer,
            "{}",
            text.style(Local::now().format_with_items(self.time_items.iter()))
        )?;

        let bg = utils::color::themed(color, self.color_depth);

        let mut on_bg = owo_colors::Style::new().on_color(bg);
        if icons.name == "nerd" {
            on_bg = on_bg.remove_bg().color(bg);
        }
        write!(
            writer,
            " {} {}{}{} {} ",
            accent.dimmed().style(icons.separator),
            on_bg.style(icons.bracket_open),
            owo_colors::Style::new().on_color(bg).style(level_label),
            on_bg.style(icons.bracket_close),
            accent.style(icons.time_bracket_close),
        )?;

        if self.show_path {
            let max_width = self.path_width;
            let line = event.metadata().line().unwrap_or(0);
            let normalized = event.metadata().file().unwrap_or("?").replace('\\', "/");
            let original = Path::new(&normalized);

            let relative: PathBuf = original
                .components()
                .skip_while(|component| component.as_os_str() != "src")
                .skip(1)
                .collect();

            let path = if relative.as_os_str().is_empty() {
                original
            } else {
                &relative
            };

            let path_str = path.to_string_lossy().replace('\\', "/");
            let full = format_compact!("{path_str}:{line}");

            let path_text = if UnicodeWidthStr::width(full.as_str()) <= max_width {
                format_compact!("{full:>max_width$}")
            } else {
                let filename = path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or(&path_str);

                let file_with_line = format_compact!("{filename}:{line}");
                let mut truncated = file_with_line.clone();

                if let Some(parent) = path.parent() {
                    for component in parent.components().rev() {
                        let component = component.as_os_str().to_string_lossy();
                        let candidate = format_compact!("{component}/{truncated}");

                        if UnicodeWidthStr::width(candidate.as_str()) > max_width {
                            break;
                        }

                        truncated = candidate;
                    }
                }

                if truncated == file_with_line {
                    let mut width = 0;
                    let mut start = full.len();

                    for (i, ch) in full.char_indices().rev() {
                        let char_width = ch.width().unwrap_or(0);

                        if width + char_width >= max_width {
                            break;
                        }

                        width += char_width;
                        start = i;
                    }

                    format_compact!("…{}", &full[start..])
                } else {
                    format_compact!("{truncated:>max_width$}")
                }
            };

            write!(writer, "{}", text.style(path_text))?;
            write!(writer, " {} ", accent.style(icons.arrow))?;
        }

        let mut visitor = EventVisitor::default();
        event.record(&mut visitor);

        let mut sep = if let Some(msg) = visitor.message {
            write!(writer, "{}", text.style(msg))?;
            " "
        } else {
            ""
        };

        for (k, v) in &visitor.fields {
            write!(
                writer,
                "{sep}{}{}{}",
                secondary.style(k),
                accent.style("="),
                text.style(v)
            )?;
            sep = " ";
        }

        if self.show_spans {
            let scope = ctx
                .event_scope()
                .or_else(|| ctx.lookup_current().map(|s| s.scope()));

            if let Some(scope) = scope {
                let mut iter = scope.from_root().peekable();

                if iter.peek().is_some() {
                    let accent_dimmed = accent.dimmed();
                    let text_dimmed = text.dimmed();

                    write!(writer, " {}", accent.style("["))?;

                    while let Some(span) = iter.next() {
                        let is_last = iter.peek().is_none();
                        let span_style = if is_last { text } else { text_dimmed };

                        write!(writer, "{}", span_style.style(span.name()))?;

                        if let Some(fields) = span.extensions().get::<FormattedFields<N>>() {
                            let fields_str = fields.fields.as_str();
                            if !fields_str.is_empty() {
                                write!(writer, " {}", span_style.style(fields_str))?;
                            }
                        }

                        if !is_last {
                            write!(writer, "{} ", accent_dimmed.style(icons.span_join))?;
                        }
                    }

                    write!(writer, "{}", accent.style("]"))?;
                }
            }
        }

        writeln!(writer)
    }
}

#[cfg(test)]
mod test;
