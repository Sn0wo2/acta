use crate::color::Styled;
use crate::config::ColorDepth;
use crate::config::{Icons, LevelLabels, Style, Theme};
use chrono::Utc;
use owo_colors::Rgb;
use owo_colors::Style as OwoStyle;
use smallvec::SmallVec;

use std::fmt;
use std::sync::Arc;

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
    pub(crate) path_width: usize,
    pub(crate) show_path: bool,
    pub(crate) show_spans: bool,
    pub(crate) style: Arc<arc_swap::ArcSwap<Style>>,
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
            path_width: DEFAULT_PATH_WIDTH,
            show_path: true,
            show_spans: true,
            style: Arc::new(arc_swap::ArcSwap::new(Arc::new(Style::default()))),
            color_depth: ColorDepth::TrueColor,
        }
    }

    /// Creates a Formatter that shares its style state with an existing handle.
    /// All builder methods (`with_theme`, `with_icons`, etc.) will write through
    /// this handle, and any future writes to the handle (e.g. via TracingGuard::with_style)
    /// are visible immediately in `format_event`.
    #[must_use]
    pub fn new_with_handle(style: Arc<arc_swap::ArcSwap<Style>>) -> Self {
        Self {
            time_format: String::from("%H:%M:%S"),
            path_width: DEFAULT_PATH_WIDTH,
            show_path: true,
            show_spans: true,
            style,
            color_depth: ColorDepth::TrueColor,
        }
    }

    /// Returns a copy of the current style configuration.
    #[must_use]
    pub fn style_config(&self) -> Style {
        **self.style.load()
    }

    #[must_use]
    pub fn with_style_config(self, new_style: Style) -> Self {
        self.style.store(Arc::new(new_style));
        self
    }

    #[must_use]
    pub fn with_icons(self, icons: Icons) -> Self {
        let mut style = **self.style.load();
        style.icons = icons;
        self.style.store(Arc::new(style));
        self
    }

    #[must_use]
    pub fn with_labels(self, labels: LevelLabels) -> Self {
        let mut style = **self.style.load();
        style.labels = labels;
        self.style.store(Arc::new(style));
        self
    }

    #[must_use]
    pub fn with_theme(self, theme: Theme) -> Self {
        let mut style = **self.style.load();
        style.theme = theme;
        self.style.store(Arc::new(style));
        self
    }

    #[must_use]
    pub const fn with_color_depth(mut self, depth: ColorDepth) -> Self {
        self.color_depth = depth;
        self
    }

    #[must_use]
    pub fn with_time_format(mut self, fmt: impl Into<String>) -> Self {
        self.time_format = fmt.into();
        self
    }

    #[must_use]
    pub const fn with_path_width(mut self, width: usize) -> Self {
        self.path_width = width;
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

    pub(crate) fn format_path(&self, file: &str, line: u32) -> String {
        let max_width = self.path_width;

        let relative = file
            .split("src/")
            .nth(1)
            .or_else(|| file.split("src\\").nth(1))
            .unwrap_or(file);

        let path_str = relative.replace('\\', "/");

        let full = format!("{path_str}:{line}");
        if full.len() <= max_width {
            return format!("{full:>max_width$}");
        }

        if let Some(last_slash) = path_str.rfind('/') {
            let filename = &path_str[last_slash + 1..];
            let file_with_line = format!("{filename}:{line}");

            if file_with_line.len() + 2 <= max_width {
                let dir_part = &path_str[..last_slash];
                let remaining = max_width.saturating_sub(file_with_line.len() + 1);
                let start = dir_part.len().saturating_sub(remaining);
                let mut adj = start;
                while adj < dir_part.len() && !dir_part.is_char_boundary(adj) {
                    adj += 1;
                }
                let dir_tail = &dir_part[adj..];
                let clean_dir = dir_tail
                    .rfind('/')
                    .map_or(dir_tail, |i| &dir_tail[i + 1..]);

                let mut result = String::with_capacity(max_width);
                use std::fmt::Write;
                let _ = write!(result, "{clean_dir}/{file_with_line}");
                return result;
            }
        }

        // Truncate from left with ellipsis, guarding char boundaries
        let start = full.len().saturating_sub(max_width.saturating_sub(1));
        let mut adj = start;
        while adj < full.len() && !full.is_char_boundary(adj) {
            adj += 1;
        }
        format!("\u{2026}{}", &full[adj..])
    }

    fn write_time(&self, writer: &mut Writer<'_>, theme: &Theme) -> fmt::Result {
        let now = Utc::now();
        write!(
            writer,
            "{}",
            OwoStyle::from(Styled::new(
                Rgb(theme.text.0, theme.text.1, theme.text.2),
                self.color_depth
            ))
            .style(now.format(&self.time_format))
        )
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
            OwoStyle::from(
                Styled::new(
                    Rgb(theme.text.0, theme.text.1, theme.text.2),
                    self.color_depth
                )
                .dimmed()
            )
            .style(self.format_path(
                event.metadata().file().unwrap_or("?"),
                event.metadata().line().unwrap_or(0),
            ))
        )?;
        write!(
            writer,
            " {} ",
            OwoStyle::from(Styled::new(
                Rgb(theme.accent.0, theme.accent.1, theme.accent.2),
                self.color_depth
            ))
            .style(icons.arrow)
        )?;
        Ok(())
    }

    fn format_fields(
        &self,
        writer: &mut Writer<'_>,
        event: &Event<'_>,
        theme: &Theme,
    ) -> fmt::Result {
        let mut visitor = EventVisitor::default();
        event.record(&mut visitor);

        let mut sep = if let Some(msg) = visitor.message {
            write!(
                writer,
                "{}",
                OwoStyle::from(Styled::new(
                    Rgb(theme.text.0, theme.text.1, theme.text.2),
                    self.color_depth
                ))
                .style(msg)
            )?;
            " "
        } else {
            ""
        };

        for (k, v) in visitor.fields {
            write!(
                writer,
                "{}{}{}{}",
                sep,
                OwoStyle::from(Styled::new(
                    Rgb(theme.secondary.0, theme.secondary.1, theme.secondary.2),
                    self.color_depth
                ))
                .style(k),
                OwoStyle::from(Styled::new(
                    Rgb(theme.accent.0, theme.accent.1, theme.accent.2),
                    self.color_depth
                ))
                .style("="),
                OwoStyle::from(Styled::new(
                    Rgb(theme.text.0, theme.text.1, theme.text.2),
                    self.color_depth
                ))
                .style(v)
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

        let total = spans.len();
        let accent = OwoStyle::from(Styled::new(
            Rgb(theme.accent.0, theme.accent.1, theme.accent.2),
            self.color_depth,
        ));
        let accent_dimmed = OwoStyle::from(
            Styled::new(
                Rgb(theme.accent.0, theme.accent.1, theme.accent.2),
                self.color_depth,
            )
            .dimmed(),
        );
        let text = OwoStyle::from(Styled::new(
            Rgb(theme.text.0, theme.text.1, theme.text.2),
            self.color_depth,
        ));
        let text_dimmed = OwoStyle::from(
            Styled::new(
                Rgb(theme.text.0, theme.text.1, theme.text.2),
                self.color_depth,
            )
            .dimmed(),
        );

        write!(writer, " {}", accent.style("["))?;

        for (i, span) in spans.iter().enumerate() {
            if i > 0 {
                write!(writer, "{} ", accent_dimmed.style(icons.span_join))?;
            }

            let span_style = if i == total - 1 { text } else { text_dimmed };

            write!(writer, "{}", span_style.style(span.name()))?;

            let extensions = span.extensions();
            if let Some(fields) = extensions.get::<FormattedFields<N>>() {
                let fields_str = fields.fields.as_str();
                if !fields_str.is_empty() {
                    write!(writer, " {}", span_style.style(fields_str))?;
                }
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
        let config = self.style.load();

        let level = event.metadata().level();

        let (color, level_label) = match *level {
            Level::ERROR => (config.theme.error, config.labels.error),
            Level::WARN => (config.theme.warn, config.labels.warn),
            Level::INFO => (config.theme.info, config.labels.info),
            Level::DEBUG => (config.theme.debug, config.labels.debug),
            Level::TRACE => (config.theme.trace, config.labels.trace),
        };

        let level_rgb = Rgb(color.0, color.1, color.2);
        let fg_only = OwoStyle::from(Styled::new(level_rgb, self.color_depth));
        let on_bg = OwoStyle::from(Styled::new(level_rgb, self.color_depth).on());

        let bracket_style = if config.icons.name == "nerd" {
            fg_only
        } else {
            on_bg
        };

        write!(
            writer,
            "{}",
            OwoStyle::from(Styled::new(
                Rgb(
                    config.theme.accent.0,
                    config.theme.accent.1,
                    config.theme.accent.2
                ),
                self.color_depth
            ))
            .style(config.icons.time_bracket_open)
        )?;
        self.write_time(&mut writer, &config.theme)?;
        write!(
            writer,
            " {} ",
            OwoStyle::from(
                Styled::new(
                    Rgb(
                        config.theme.accent.0,
                        config.theme.accent.1,
                        config.theme.accent.2
                    ),
                    self.color_depth
                )
                .dimmed()
            )
            .style(config.icons.separator)
        )?;

        write!(writer, "{}", bracket_style.style(config.icons.bracket_open))?;
        write!(writer, "{}", on_bg.style(level_label))?;
        write!(
            writer,
            "{} ",
            bracket_style.style(config.icons.bracket_close)
        )?;

        write!(
            writer,
            "{} ",
            OwoStyle::from(Styled::new(
                Rgb(
                    config.theme.accent.0,
                    config.theme.accent.1,
                    config.theme.accent.2
                ),
                self.color_depth
            ))
            .style(config.icons.time_bracket_close)
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
