use crate::config::ColorDepth;
use crate::config::Style;
use crate::config::Theme;
use chrono::Local;
use chrono::format::Item;
use chrono::format::StrftimeItems;
use compact_str::CompactString;
use std::fmt;
use std::fmt::Write as _;
use std::sync::Arc;
use tracing::{Event, Level, Subscriber};
use tracing_subscriber::Registry;
use tracing_subscriber::fmt::FormattedFields;
use tracing_subscriber::fmt::format::DefaultFields;
use tracing_subscriber::fmt::format::Writer;
use tracing_subscriber::fmt::{FmtContext, FormatEvent};
use tracing_subscriber::registry::LookupSpan;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

pub(crate) mod stacktrace;
mod visitor;
use visitor::EventVisitor;

pub(crate) struct CustomFormatter(
    pub(crate) Arc<dyn FormatEvent<Registry, DefaultFields> + Send + Sync>,
);

impl FormatEvent<Registry, DefaultFields> for CustomFormatter {
    fn format_event(
        &self,
        ctx: &FmtContext<'_, Registry, DefaultFields>,
        writer: Writer<'_>,
        event: &Event<'_>,
    ) -> fmt::Result {
        self.0.format_event(ctx, writer, event)
    }
}

const DEFAULT_PATH_WIDTH: usize = include!(concat!(env!("OUT_DIR"), "/path_width"));

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct Formatter {
    style: Style,
    color_depth: ColorDepth,
    colors: [owo_colors::DynColors; 14],
    time_items: Vec<Item<'static>>,
    #[cfg(feature = "serde")]
    time_format: Option<String>,
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
            colors: [owo_colors::DynColors::Ansi(owo_colors::AnsiColors::Default); 14],
            time_items: StrftimeItems::new("%H:%M:%S").map(Item::to_owned).collect(),
            #[cfg(feature = "serde")]
            time_format: None,
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
        #[cfg(feature = "serde")]
        {
            self.time_format = Some(fmt.as_ref().to_owned());
        }
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
        let [time, error_bg, warn_bg, info_bg, debug_bg, trace_bg] =
            [accent, error, warn, info, debug, trace].map(|color| {
                <(u8, u8, u8)>::from(
                    <[u8; 3]>::from(color).map(|channel| (u16::from(channel) * 4 / 5) as u8),
                )
            });
        let time = <(u8, u8, u8)>::from(<[u8; 3]>::from(time).map(|channel| channel - channel / 4));
        self.colors = [
            accent, text, secondary, error, warn, info, debug, trace, time, error_bg, warn_bg,
            info_bg, debug_bg, trace_bg,
        ]
        .map(|(r, g, b)| match self.color_depth {
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

#[cfg(feature = "serde")]
impl serde::Serialize for Formatter {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;

        let mut state = serializer.serialize_struct("Formatter", 5)?;
        state.serialize_field("time_format", &self.time_format)?;
        state.serialize_field("path_width", &self.path_width)?;
        state.serialize_field("show_path", &self.show_path)?;
        state.serialize_field("show_spans", &self.show_spans)?;
        state.serialize_field("color_depth", &self.color_depth)?;
        state.end()
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for Formatter {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(serde::Deserialize)]
        struct Fields {
            time_format: Option<String>,
            path_width: Option<usize>,
            show_path: Option<bool>,
            show_spans: Option<bool>,
            color_depth: Option<ColorDepth>,
        }

        let fields = Fields::deserialize(deserializer)?;
        let mut formatter = Self::new();
        if let Some(time_format) = fields.time_format {
            formatter.time_items = StrftimeItems::new(&time_format)
                .map(Item::to_owned)
                .collect();
            formatter.time_format = Some(time_format);
        }
        if let Some(path_width) = fields.path_width {
            formatter.path_width = path_width;
        }
        if let Some(show_path) = fields.show_path {
            formatter.show_path = show_path;
        }
        if let Some(show_spans) = fields.show_spans {
            formatter.show_spans = show_spans;
        }
        if let Some(color_depth) = fields.color_depth {
            formatter.color_depth = color_depth;
            formatter.update_colors();
        }
        Ok(formatter)
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
        let [
            accent,
            text,
            secondary,
            error,
            warn,
            info,
            debug,
            trace,
            time_bg,
            error_bg,
            warn_bg,
            info_bg,
            debug_bg,
            trace_bg,
        ] = self.colors;
        let ansi = writer.has_ansi_escapes() && self.color_depth != ColorDepth::NoColor;
        let nerd = cfg!(feature = "nerd") && icons.name == "nerd";
        let decorated = icons.name == "unicode" || nerd;
        let frame_space = if decorated { " " } else { "" };
        let icon_space = if nerd { " " } else { "" };

        let (level_color, bg, level_label) = match *event.metadata().level() {
            Level::ERROR => (error, error_bg, self.style.labels.error),
            Level::WARN => (warn, warn_bg, self.style.labels.warn),
            Level::INFO => (info, info_bg, self.style.labels.info),
            Level::DEBUG => (debug, debug_bg, self.style.labels.debug),
            Level::TRACE => (trace, trace_bg, self.style.labels.trace),
        };
        let [accent, text, secondary] = [accent, text, secondary].map(|color| {
            if ansi {
                owo_colors::Style::new().color(color)
            } else {
                owo_colors::Style::new()
            }
        });
        let accent_dimmed = if ansi { accent.dimmed() } else { accent };
        let background = if ansi {
            owo_colors::Style::new()
                .on_color(bg)
                .color(owo_colors::AnsiColors::BrightWhite)
        } else {
            owo_colors::Style::new()
        };
        let level_outline = if ansi {
            owo_colors::Style::new().color(level_color)
        } else {
            owo_colors::Style::new()
        };
        #[cfg(feature = "nerd")]
        let (clock_icon, level_icon, file_icon) = if nerd {
            use nerd_font_symbols::fa;
            (
                fa::FA_CLOCK,
                match *event.metadata().level() {
                    Level::ERROR => fa::FA_CIRCLE_XMARK,
                    Level::WARN => fa::FA_TRIANGLE_EXCLAMATION,
                    Level::INFO => fa::FA_CIRCLE_INFO,
                    Level::DEBUG => fa::FA_BUG,
                    Level::TRACE => fa::FA_BINOCULARS,
                },
                fa::FA_FILE,
            )
        } else {
            ("", "", "")
        };
        #[cfg(not(feature = "nerd"))]
        let (clock_icon, level_icon, file_icon) = ("", "", "");
        let now = Local::now();
        let timestamp = now.format_with_items(self.time_items.iter());
        if nerd && ansi {
            write!(
                writer,
                "{}{}{}{}{}  ",
                owo_colors::Style::new()
                    .color(time_bg)
                    .style(icons.time_bracket_open),
                owo_colors::Style::new()
                    .on_color(time_bg)
                    .color(owo_colors::AnsiColors::BrightWhite)
                    .style(format_args!(" {clock_icon} {timestamp} ")),
                owo_colors::Style::new()
                    .color(time_bg)
                    .on_color(bg)
                    .style(icons.separator),
                background.style(format_args!(" {level_icon} {level_label} ")),
                owo_colors::Style::new()
                    .color(bg)
                    .style(icons.time_bracket_close),
            )?;
        } else {
            let (time_open, time_close, separator) = if nerd {
                ("❬", "❭", "┊")
            } else {
                (
                    icons.time_bracket_open,
                    icons.time_bracket_close,
                    icons.separator,
                )
            };
            write!(
                writer,
                "{}{frame_space}{clock_icon}{icon_space}{} {} {}{}{} {}{frame_space} ",
                accent.style(time_open),
                text.style(timestamp),
                accent_dimmed.style(separator),
                level_outline.style(icons.bracket_open),
                background.style(format_args!("{level_icon}{icon_space}{level_label}")),
                level_outline.style(icons.bracket_close),
                accent.style(time_close),
            )?;
        }

        if self.show_path {
            if nerd {
                write!(writer, "{} ", accent_dimmed.style(file_icon))?;
            }
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
                (if ansi { text.dimmed() } else { text }).style(format_args!(
                    "{:padding$}{}{path_text}",
                    "",
                    if ellipsis { "…" } else { "" },
                    padding = max_width
                        .saturating_sub(UnicodeWidthStr::width(path_text) + usize::from(ellipsis))
                ))
            )?;
            write!(writer, " {} ", secondary.style(icons.arrow))?;
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
                let (span_open, span_close) = if decorated {
                    ("⟨", "⟩")
                } else {
                    ("[", "]")
                };
                write!(writer, " {}{frame_space}", secondary.style(span_open))?;

                while let Some(span) = iter.next() {
                    let is_last = iter.peek().is_none();
                    let span_style = if is_last || !ansi {
                        secondary
                    } else {
                        secondary.dimmed()
                    };

                    write!(writer, "{}", span_style.style(span.name()))?;

                    if let Some(fields) = span.extensions().get::<FormattedFields<N>>() {
                        let fields_str = fields.fields.as_str();
                        if !fields_str.is_empty() {
                            write!(writer, " {}", span_style.style(fields_str))?;
                        }
                    }

                    if !is_last {
                        write!(
                            writer,
                            "{frame_space}{} ",
                            accent_dimmed.style(icons.span_join)
                        )?;
                    }
                }

                write!(writer, "{frame_space}{}", secondary.style(span_close))?;
            }
        }

        writeln!(writer)
    }
}

#[cfg(test)]
mod test;
