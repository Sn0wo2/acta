use crate::color::AnsiStyle;
use crate::config::ColorDepth;
use crate::config::{Icons, LevelLabels, Style, Theme};
use chrono::Utc;
use compact_str::{CompactString, format_compact};
use owo_colors::Rgb;
use owo_colors::Style as OwoStyle;
use smallvec::SmallVec;

use std::fmt;

use tracing::{Event, Level, Subscriber};
use tracing_subscriber::fmt::FormattedFields;
use tracing_subscriber::fmt::format::Writer;
use tracing_subscriber::fmt::{FmtContext, FormatEvent};
use tracing_subscriber::registry::LookupSpan;

mod visitor;
use visitor::EventVisitor;

const DEFAULT_PATH_WIDTH: usize = include!(concat!(env!("OUT_DIR"), "/path_width"));

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct Formatter {
    pub(crate) time_format: String,
    /// Pre-parsed `time_format`; `None` falls back to runtime parsing.
    pub(crate) time_items: Option<Vec<chrono::format::Item<'static>>>,
    pub(crate) path_width: usize,
    pub(crate) show_path: bool,
    pub(crate) show_spans: bool,
    pub(crate) style: Style,
    pub(crate) color_depth: ColorDepth,
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
            time_format: String::from("%H:%M:%S"),
            time_items: Self::parse_time_items("%H:%M:%S"),
            path_width: DEFAULT_PATH_WIDTH,
            show_path: true,
            show_spans: true,
            style: Style::default(),
            color_depth: ColorDepth::TrueColor,
        }
    }

    /// Returns a copy of the current style configuration.
    #[must_use]
    pub const fn style_config(&self) -> Style {
        self.style
    }

    #[must_use]
    pub const fn with_style_config(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    fn update_style(mut self, f: impl FnOnce(&mut Style)) -> Self {
        f(&mut self.style);
        self
    }

    #[must_use]
    pub fn with_icons(self, icons: Icons) -> Self {
        self.update_style(|s| s.icons = icons)
    }

    #[must_use]
    pub fn with_labels(self, labels: LevelLabels) -> Self {
        self.update_style(|s| s.labels = labels)
    }

    #[must_use]
    pub fn with_theme(self, theme: Theme) -> Self {
        self.update_style(|s| s.theme = theme)
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

    #[must_use]
    pub fn with_time_format(mut self, fmt: impl Into<String>) -> Self {
        self.time_format = fmt.into();
        self.time_items = Self::parse_time_items(&self.time_format);
        self
    }

    fn parse_time_items(fmt: &str) -> Option<Vec<chrono::format::Item<'static>>> {
        chrono::format::StrftimeItems::new(fmt)
            .parse_to_owned()
            .ok()
    }

    fn themed(&self, (r, g, b): (u8, u8, u8)) -> OwoStyle {
        OwoStyle::from(AnsiStyle::new(Rgb(r, g, b), self.color_depth))
    }

    fn themed_dimmed(&self, (r, g, b): (u8, u8, u8)) -> OwoStyle {
        OwoStyle::from(AnsiStyle::new(Rgb(r, g, b), self.color_depth).dimmed())
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

    fn format_path(&self, file: &str, line: u32) -> CompactString {
        let max_width = self.path_width;
        let relative = file
            .split_once("src/")
            .map(|(_, tail)| tail)
            .or_else(|| file.split_once("src\\").map(|(_, tail)| tail))
            .unwrap_or(file);

        let path_str = if relative.contains('\\') {
            relative.replace('\\', "/").into()
        } else {
            CompactString::new(relative)
        };

        let full = format_compact!("{path_str}:{line}");
        if full.len() <= max_width {
            return format_compact!("{full:>max_width$}");
        }

        if let Some(last_slash) = path_str.rfind('/') {
            let filename = &path_str[last_slash + 1..];
            let file_with_line = format_compact!("{filename}:{line}");

            if file_with_line.len() + 2 <= max_width {
                let dir_part = &path_str[..last_slash];
                let mut start = dir_part
                    .len()
                    .saturating_sub(max_width.saturating_sub(file_with_line.len() + 1));
                while start < dir_part.len() && !dir_part.is_char_boundary(start) {
                    start += 1;
                }
                let dir_tail = &dir_part[start..];
                let clean_dir =
                    if start > 0 && dir_part.as_bytes().get(start - 1).copied() == Some(b'/') {
                        dir_tail
                    } else {
                        dir_tail.find('/').map_or(dir_tail, |i| &dir_tail[i + 1..])
                    };

                let formatted = format_compact!("{clean_dir}/{file_with_line}");
                return format_compact!("{formatted:>max_width$}");
            }
        }

        // Truncate from left with ellipsis, guarding char boundaries
        let mut adj = full.len().saturating_sub(max_width.saturating_sub(1));
        while adj < full.len() && !full.is_char_boundary(adj) {
            adj += 1;
        }
        format_compact!("…{}", &full[adj..])
    }

    fn write_time(&self, writer: &mut Writer<'_>, theme: &Theme) -> fmt::Result {
        let now = Utc::now();
        let style = self.themed(theme.text);
        match &self.time_items {
            Some(items) => write!(
                writer,
                "{}",
                style.style(now.format_with_items(items.iter()))
            ),
            None => write!(writer, "{}", style.style(now.format(&self.time_format))),
        }
    }

    fn format_path_section(
        &self,
        writer: &mut Writer<'_>,
        event: &Event<'_>,
        theme: &Theme,
        icons: &Icons,
    ) -> fmt::Result {
        write!(
            writer,
            "{}",
            self.themed_dimmed(theme.text).style(self.format_path(
                event.metadata().file().unwrap_or("?"),
                event.metadata().line().unwrap_or(0),
            ))
        )?;
        write!(writer, " {} ", self.themed(theme.accent).style(icons.arrow))
    }

    fn format_fields(
        &self,
        writer: &mut Writer<'_>,
        event: &Event<'_>,
        theme: &Theme,
    ) -> fmt::Result {
        let mut visitor = EventVisitor::default();
        event.record(&mut visitor);

        let key_style = self.themed(theme.secondary);
        let eq_style = self.themed(theme.accent);
        let value_style = self.themed(theme.text);

        let mut sep = if let Some(msg) = visitor.message {
            write!(writer, "{}", value_style.style(msg))?;
            " "
        } else {
            ""
        };

        for (k, v) in &visitor.fields {
            write!(
                writer,
                "{sep}{}{}{}",
                key_style.style(k),
                eq_style.style("="),
                value_style.style(v)
            )?;
            sep = " ";
        }

        Ok(())
    }

    fn format_spans<S, N>(
        &self,
        writer: &mut Writer<'_>,
        ctx: &FmtContext<'_, S, N>,
        theme: &Theme,
        icons: &Icons,
    ) -> fmt::Result
    where
        S: Subscriber + for<'a> LookupSpan<'a>,
        N: for<'a> tracing_subscriber::fmt::FormatFields<'a> + 'static,
    {
        let Some(scope) = ctx
            .event_scope()
            .or_else(|| ctx.lookup_current().map(|s| s.scope()))
        else {
            return Ok(());
        };

        let spans: SmallVec<[_; 8]> = scope.from_root().collect();
        if spans.is_empty() {
            return Ok(());
        }

        let accent = self.themed(theme.accent);
        let accent_dimmed = self.themed_dimmed(theme.accent);
        let text = self.themed(theme.text);
        let text_dimmed = self.themed_dimmed(theme.text);

        write!(writer, " {}", accent.style("["))?;

        let mut iter = spans.iter().peekable();

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

        write!(writer, "{}", accent.style("]"))
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

        let level = event.metadata().level();

        let (color, level_label) = match *level {
            Level::ERROR => (config.theme.error, config.labels.error),
            Level::WARN => (config.theme.warn, config.labels.warn),
            Level::INFO => (config.theme.info, config.labels.info),
            Level::DEBUG => (config.theme.debug, config.labels.debug),
            Level::TRACE => (config.theme.trace, config.labels.trace),
        };

        let on_bg =
            OwoStyle::from(AnsiStyle::new(Rgb(color.0, color.1, color.2), self.color_depth).on());
        let bracket_style = if config.icons.name == "nerd" {
            self.themed(color)
        } else {
            on_bg
        };
        let accent = self.themed(config.theme.accent);
        let accent_dimmed = self.themed_dimmed(config.theme.accent);

        write!(writer, "{}", accent.style(config.icons.time_bracket_open))?;
        self.write_time(&mut writer, &config.theme)?;
        write!(
            writer,
            " {} {}{}{} {} ",
            accent_dimmed.style(config.icons.separator),
            bracket_style.style(config.icons.bracket_open),
            on_bg.style(level_label),
            bracket_style.style(config.icons.bracket_close),
            accent.style(config.icons.time_bracket_close),
        )?;

        if self.show_path {
            self.format_path_section(&mut writer, event, &config.theme, &config.icons)?;
        }

        self.format_fields(&mut writer, event, &config.theme)?;

        if self.show_spans {
            self.format_spans(&mut writer, ctx, &config.theme, &config.icons)?;
        }

        writeln!(writer)
    }
}

#[cfg(test)]
mod test;
