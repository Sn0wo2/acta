use crate::config::ColorDepth;
use crate::config::Style;
use crate::config::Theme;
use chrono::Local;
use chrono::format::Item;
use chrono::format::StrftimeItems;
use compact_str::CompactString;
use std::fmt;
use std::fmt::Write as _;
use tracing::{Event, Level, Subscriber};
use tracing_subscriber::fmt::FormattedFields;
use tracing_subscriber::fmt::format::Writer;
use tracing_subscriber::fmt::{FmtContext, FormatEvent};
use tracing_subscriber::registry::LookupSpan;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

mod visitor;
use visitor::EventVisitor;

const DEFAULT_PATH_WIDTH: usize = include!(concat!(env!("OUT_DIR"), "/path_width"));

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct Formatter {
    style: Style,
    color_depth: ColorDepth,
    colors: [owo_colors::DynColors; 8],
    time_items: Vec<Item<'static>>,
    path_width: usize,
    show_path: bool,
    show_spans: bool,
}

impl Default for Formatter {
    fn default() -> Self {
        Self::new()
    }
}

impl Formatter {
    #[must_use]
    pub fn new() -> Self {
        let mut formatter = Self {
            style: Style::default(),
            color_depth: ColorDepth::TrueColor,
            colors: [owo_colors::DynColors::Ansi(owo_colors::AnsiColors::Default); 8],
            time_items: StrftimeItems::new("%H:%M:%S").map(Item::to_owned).collect(),
            path_width: DEFAULT_PATH_WIDTH,
            show_path: true,
            show_spans: true,
        };
        formatter.update_colors();
        formatter
    }

    #[must_use]
    pub fn with_style(mut self, style: Style) -> Self {
        self.style = style;
        self.update_colors();
        self
    }

    #[must_use]
    pub fn with_theme(mut self, theme: Theme) -> Self {
        self.style.theme = theme;
        self.update_colors();
        self
    }

    #[must_use]
    pub fn with_color_depth(mut self, depth: ColorDepth) -> Self {
        self.color_depth = depth;
        self.update_colors();
        self
    }

    #[must_use]
    pub const fn with_path_width(mut self, width: usize) -> Self {
        self.path_width = width;
        self
    }

    /// Timestamps use the local system timezone.
    #[must_use]
    pub fn with_time_format(mut self, fmt: impl AsRef<str>) -> Self {
        self.time_items = StrftimeItems::new(fmt.as_ref())
            .map(Item::to_owned)
            .collect();
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

    fn update_colors(&mut self) {
        use owo_colors::{AnsiColors, DynColors, XtermColors};

        let Theme {
            accent,
            text,
            secondary,
            error,
            warn,
            info,
            debug,
            trace,
        } = self.style.theme;
        self.colors =
            [accent, text, secondary, error, warn, info, debug, trace].map(|(r, g, b)| match self
                .color_depth
            {
                ColorDepth::TrueColor => DynColors::Rgb(r, g, b),
                ColorDepth::Ansi256 => {
                    DynColors::Xterm(XtermColors::from(ansi_colours::ansi256_from_rgb((r, g, b))))
                }
                ColorDepth::Ansi16 => DynColors::Ansi(
                    [
                        AnsiColors::Black,
                        AnsiColors::Red,
                        AnsiColors::Green,
                        AnsiColors::Yellow,
                        AnsiColors::Blue,
                        AnsiColors::Magenta,
                        AnsiColors::Cyan,
                        AnsiColors::White,
                        AnsiColors::BrightBlack,
                        AnsiColors::BrightRed,
                        AnsiColors::BrightGreen,
                        AnsiColors::BrightYellow,
                        AnsiColors::BrightBlue,
                        AnsiColors::BrightMagenta,
                        AnsiColors::BrightCyan,
                        AnsiColors::BrightWhite,
                    ]
                    .get(anstyle_lossy::rgb_to_ansi(
                        (r, g, b).into(),
                        anstyle_lossy::palette::Palette::default(),
                    ) as usize)
                    .copied()
                    .unwrap_or(AnsiColors::White),
                ),
                ColorDepth::NoColor => DynColors::Ansi(AnsiColors::Default),
            });
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
        let icons = &self.style.icons;
        let [accent, text, secondary, error, warn, info, debug, trace] = self.colors;
        let ansi = writer.has_ansi_escapes() && self.color_depth != ColorDepth::NoColor;

        let (bg, level_label) = match *event.metadata().level() {
            Level::ERROR => (error, self.style.labels.error),
            Level::WARN => (warn, self.style.labels.warn),
            Level::INFO => (info, self.style.labels.info),
            Level::DEBUG => (debug, self.style.labels.debug),
            Level::TRACE => (trace, self.style.labels.trace),
        };
        let [accent, text, secondary] = [accent, text, secondary].map(|color| {
            if ansi {
                owo_colors::Style::new().color(color)
            } else {
                owo_colors::Style::new()
            }
        });
        let accent_dimmed = if ansi { accent.dimmed() } else { accent };

        write!(writer, "{}", accent.style(icons.time_bracket_open))?;
        write!(
            writer,
            "{}",
            text.style(Local::now().format_with_items(self.time_items.iter()))
        )?;

        let background = if ansi {
            owo_colors::Style::new().on_color(bg)
        } else {
            owo_colors::Style::new()
        };
        let on_bg = if ansi && icons.name == "nerd" {
            background.remove_bg().color(bg)
        } else {
            background
        };
        write!(
            writer,
            " {} {}{}{} {} ",
            accent_dimmed.style(icons.separator),
            on_bg.style(icons.bracket_open),
            background.style(level_label),
            on_bg.style(icons.bracket_close),
            accent.style(icons.time_bracket_close),
        )?;

        if self.show_path {
            let max_width = self.path_width;
            let source = event.metadata().file().unwrap_or("?");

            let mut tail = None;
            let mut rest = source;
            while let Some(index) = rest.find(['/', '\\']) {
                let (component, remainder) = (&rest[..index], &rest[index + 1..]);
                if component == "src" {
                    tail = Some(remainder);
                    break;
                }
                rest = remainder;
            }
            let tail = tail.filter(|tail| !tail.is_empty()).unwrap_or(source);

            let mut full = CompactString::with_capacity(tail.len() + 5);
            full.extend(tail.chars().map(|c| if c == '\\' { '/' } else { c }));
            write!(full, ":{}", event.metadata().line().unwrap_or(0))?;
            let mut start = 0;
            let mut ellipsis = false;
            if UnicodeWidthStr::width(full.as_str()) > max_width {
                start = full.rfind('/').map_or(0, |index| index + 1);
                let filename_start = start;
                let mut width = UnicodeWidthStr::width(&full[start..]);
                for component in full[..start.saturating_sub(1)].rsplit('/') {
                    let candidate_width = width + UnicodeWidthStr::width(component) + 1;
                    if candidate_width > max_width {
                        break;
                    }
                    start = start.saturating_sub(component.len() + 1);
                    width = candidate_width;
                }
                if start == filename_start {
                    ellipsis = max_width > 0;
                    start = full.len();
                    width = usize::from(ellipsis);
                    for (index, ch) in full.char_indices().rev() {
                        let char_width = ch.width().unwrap_or(0);
                        if width + char_width > max_width {
                            break;
                        }
                        width += char_width;
                        start = index;
                    }
                }
            }
            let path_text = &full[start..];
            write!(
                writer,
                "{}",
                text.style(format_args!(
                    "{:padding$}{}{path_text}",
                    "",
                    if ellipsis { "…" } else { "" },
                    padding = max_width
                        .saturating_sub(UnicodeWidthStr::width(path_text) + usize::from(ellipsis))
                ))
            )?;
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

        if self.show_spans
            && let Some(scope) = ctx
                .event_scope()
                .or_else(|| ctx.lookup_current().map(|s| s.scope()))
        {
            let mut iter = scope.from_root().peekable();

            if iter.peek().is_some() {
                write!(writer, " {}", accent.style("["))?;

                while let Some(span) = iter.next() {
                    let is_last = iter.peek().is_none();
                    let span_style = if is_last || !ansi {
                        text
                    } else {
                        text.dimmed()
                    };

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

        writeln!(writer)
    }
}

#[cfg(test)]
mod test;
