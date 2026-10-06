#[cfg(feature = "nerd")]
use nerd_font_symbols::{fa, ple};
#[cfg(feature = "file")]
use std::path::PathBuf;
use std::sync::Arc;
use tracing_subscriber::Registry;
use tracing_subscriber::fmt::format::DefaultFields;
use tracing_subscriber::fmt::writer::BoxMakeWriter;
use tracing_subscriber::fmt::{FormatEvent, MakeWriter};

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ColorDepth {
    TrueColor,
    Ansi256,
    Ansi16,
    NoColor,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum LevelAlignment {
    Left,
    Right,
}

#[macro_export]
macro_rules! level_labels {
    ($labels:expr $(,)?) => {
        $crate::level_labels!($labels, $crate::LevelAlignment::Left)
    };
    ($labels:expr, $alignment:expr $(,)?) => {
        $crate::level_labels!(@align $labels, $alignment; error, warn, info, debug, trace)
    };
    (@align $labels:expr, $alignment:expr; $($field:ident),+) => {{
        #[allow(clippy::indexing_slicing, clippy::panic)]
        const {
            const __ACTA_LABELS: $crate::LevelLabels = $labels;
            const __ACTA_WIDTH: usize = {
                let mut width = 0;
                $(let len = __ACTA_LABELS.$field.trim_ascii().len(); if len > width { width = len; })+
                width
            };
            $crate::LevelLabels::custom($(match ::core::str::from_utf8(&const {
                let bytes = __ACTA_LABELS.$field.trim_ascii().as_bytes();
                assert!(bytes.is_ascii(), "compile-time label alignment requires ASCII labels");
                let mut padded = [b' '; __ACTA_WIDTH];
                let start = if let $crate::LevelAlignment::Right = $alignment { __ACTA_WIDTH - bytes.len() } else { 0 };
                let mut i = 0; while i < bytes.len() { padded[start + i] = bytes[i]; i += 1; }
                padded
            }) { Ok(label) => label, Err(_) => panic!("aligned labels must be ASCII") }),+)
        }
    }};
}

#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct LevelLabels {
    pub error: &'static str,
    pub warn: &'static str,
    pub info: &'static str,
    pub debug: &'static str,
    pub trace: &'static str,
}

impl LevelLabels {
    pub const fn custom(
        error: &'static str,
        warn: &'static str,
        info: &'static str,
        debug: &'static str,
        trace: &'static str,
    ) -> Self {
        Self {
            error,
            warn,
            info,
            debug,
            trace,
        }
    }

    pub const LONG: Self = crate::level_labels!(LevelLabels::custom(
        "ERROR", "WARN", "INFO", "DEBUG", "TRACE"
    ));

    pub const MEDIUM: Self = Self {
        error: "ERR",
        warn: "WRN",
        info: "INF",
        debug: "DBG",
        trace: "TRC",
    };

    pub const SHORT: Self = Self {
        error: "E",
        warn: "W",
        info: "I",
        debug: "D",
        trace: "T",
    };
}

impl Default for LevelLabels {
    fn default() -> Self {
        Self::SHORT
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct Icons {
    pub name: &'static str,
    pub bracket_open: &'static str,
    pub bracket_close: &'static str,
    pub time_bracket_open: &'static str,
    pub time_bracket_close: &'static str,
    pub separator: &'static str,
    pub arrow: &'static str,
    pub span_join: &'static str,
}

impl Icons {
    #[allow(clippy::too_many_arguments)]
    pub const fn custom(
        name: &'static str,
        bracket_open: &'static str,
        bracket_close: &'static str,
        time_bracket_open: &'static str,
        time_bracket_close: &'static str,
        separator: &'static str,
        arrow: &'static str,
        span_join: &'static str,
    ) -> Self {
        Self {
            name,
            bracket_open,
            bracket_close,
            time_bracket_open,
            time_bracket_close,
            separator,
            arrow,
            span_join,
        }
    }

    pub const UNICODE: Self = Self {
        name: "unicode",
        bracket_open: "(",
        bracket_close: ")",
        time_bracket_open: "❬",
        time_bracket_close: "❭",
        separator: "┊",
        arrow: "⟶",
        span_join: "▸",
    };

    #[cfg(feature = "nerd")]
    pub const NERD: Self = Self {
        name: "nerd",
        bracket_open: "(",
        bracket_close: ")",
        time_bracket_open: ple::PLE_LEFT_HALF_CIRCLE_THICK,
        time_bracket_close: ple::PLE_RIGHT_HALF_CIRCLE_THICK,
        separator: ple::PL_LEFT_HARD_DIVIDER,
        arrow: fa::FA_ANGLES_RIGHT,
        span_join: fa::FA_ANGLE_RIGHT,
    };
}

impl Default for Icons {
    fn default() -> Self {
        Self::UNICODE
    }
}

type Rgb = (u8, u8, u8);

#[derive(Clone, Copy, Debug)]
#[allow(clippy::exhaustive_structs)]
pub struct Theme {
    pub accent: Rgb,
    pub secondary: Rgb,
    pub text: Rgb,
    pub error: Rgb,
    pub warn: Rgb,
    pub info: Rgb,
    pub debug: Rgb,
    pub trace: Rgb,
}

impl Theme {
    #[allow(clippy::too_many_arguments)]
    const fn from_palette(
        accent: Rgb,
        secondary: Rgb,
        text: Rgb,
        error: Rgb,
        warn: Rgb,
        info: Rgb,
        debug: Rgb,
        trace: Rgb,
    ) -> Self {
        Self {
            accent,
            secondary,
            text,
            error,
            warn,
            info,
            debug,
            trace,
        }
    }
    #[rustfmt::skip]
    pub const fn acta() -> Self {
        const LIGHT_BLUE:  Rgb = (91, 206, 250);  // #5BCEFA
        const PINK:        Rgb = (245, 169, 184); // #F5A9B8
        const WHITE:       Rgb = (255, 255, 255); // #FFFFFF
        const BRIGHT_RED:  Rgb = (255, 85, 85);   // #FF5555
        const GOLD:        Rgb = (255, 200, 60);  // #FFC83C
        const OFF_WHITE:   Rgb = (240, 240, 240); // #F0F0F0
        Self::from_palette(LIGHT_BLUE, PINK, WHITE, BRIGHT_RED, GOLD, LIGHT_BLUE, PINK, OFF_WHITE)
    }

    #[rustfmt::skip]
    pub const fn monokai() -> Self {
        const CYAN:       Rgb = (102, 217, 239); // #66D9EF
        const PINK:       Rgb = (249, 38, 114);  // #F92672
        const WHITE:      Rgb = (248, 248, 242); // #F8F8F2
        const BRIGHT_RED: Rgb = (255, 85, 85);   // #FF5555
        const GOLD:       Rgb = (255, 200, 60);  // #FFC83C
        const GRAY:       Rgb = (180, 180, 180); // #B4B4B4
        Self::from_palette(CYAN, PINK, WHITE, BRIGHT_RED, GOLD, CYAN, PINK, GRAY)
    }

    #[rustfmt::skip]
    pub const fn dracula() -> Self {
        const CYAN:       Rgb = (139, 233, 253); // #8BE9FD
        const PINK:       Rgb = (255, 121, 198); // #FF79C6
        const WHITE:      Rgb = (248, 248, 242); // #F8F8F2
        const BRIGHT_RED: Rgb = (255, 85, 85);   // #FF5555
        const GOLD:       Rgb = (255, 200, 60);  // #FFC83C
        const GRAY:       Rgb = (180, 180, 180); // #B4B4B4
        Self::from_palette(CYAN, PINK, WHITE, BRIGHT_RED, GOLD, CYAN, PINK, GRAY)
    }

    #[rustfmt::skip]
    pub const fn nord() -> Self {
        const BLUE:   Rgb = (136, 192, 208); // #88C0D0
        const GREEN:  Rgb = (163, 190, 140); // #A3BE8C
        const WHITE:  Rgb = (216, 222, 233); // #D8DEE9
        const RED:    Rgb = (191, 97, 106);  // #BF616A
        const YELLOW: Rgb = (235, 203, 139); // #EBCB8B
        const GRAY:   Rgb = (180, 180, 180); // #B4B4B4
        Self::from_palette(BLUE, GREEN, WHITE, RED, YELLOW, BLUE, GREEN, GRAY)
    }

    #[rustfmt::skip]
    pub const fn catppuccin_mocha() -> Self {
        const BLUE:   Rgb = (137, 180, 250); // #89B4FA
        const MAUVE:  Rgb = (203, 166, 247); // #CBA6F7
        const TEXT:   Rgb = (205, 214, 244); // #CDD6F4
        const RED:    Rgb = (243, 139, 168); // #F38BA8
        const YELLOW: Rgb = (249, 226, 175); // #F9E2AF
        const GRAY:   Rgb = (180, 180, 180); // #B4B4B4
        Self::from_palette(BLUE, MAUVE, TEXT, RED, YELLOW, BLUE, MAUVE, GRAY)
    }

    #[rustfmt::skip]
    pub const fn gruvbox() -> Self {
        const AQUA:   Rgb = (131, 165, 152); // #83A598
        const ORANGE: Rgb = (254, 128, 25);  // #FE8019
        const LIGHT:  Rgb = (235, 219, 178); // #EBDBB2
        const RED:    Rgb = (251, 73, 52);   // #FB4934
        const YELLOW: Rgb = (250, 189, 47);  // #FABD2F
        const GRAY:   Rgb = (180, 180, 180); // #B4B4B4
        Self::from_palette(AQUA, ORANGE, LIGHT, RED, YELLOW, AQUA, ORANGE, GRAY)
    }

    #[rustfmt::skip]
    pub const fn one_dark() -> Self {
        const BLUE:   Rgb = (97, 175, 239);  // #61AFEF
        const PURPLE: Rgb = (198, 120, 221); // #C678DD
        const WHITE:  Rgb = (171, 178, 191); // #ABB2BF
        const RED:    Rgb = (224, 108, 117); // #E06C75
        const YELLOW: Rgb = (229, 192, 123); // #E5C07B
        const GRAY:   Rgb = (180, 180, 180); // #B4B4B4
        Self::from_palette(BLUE, PURPLE, WHITE, RED, YELLOW, BLUE, PURPLE, GRAY)
    }

    #[rustfmt::skip]
    pub const fn tokyo_night() -> Self {
        const BLUE:   Rgb = (122, 162, 247); // #7AA2F7
        const PURPLE: Rgb = (187, 154, 247); // #BB9AF7
        const WHITE:  Rgb = (192, 202, 245); // #C0CAF5
        const RED:    Rgb = (247, 118, 142); // #F7768E
        const YELLOW: Rgb = (224, 175, 104); // #E0AF68
        const GRAY:   Rgb = (180, 180, 180); // #B4B4B4
        Self::from_palette(BLUE, PURPLE, WHITE, RED, YELLOW, BLUE, PURPLE, GRAY)
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::acta()
    }
}

#[derive(Clone, Copy, Debug)]
#[allow(clippy::exhaustive_structs)]
pub struct Style {
    pub theme: Theme,
    pub icons: Icons,
    pub labels: LevelLabels,
}

impl Default for Style {
    fn default() -> Self {
        Self {
            theme: Theme::default(),
            icons: Icons::default(),
            labels: LevelLabels::LONG,
        }
    }
}

#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(default)
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(clippy::exhaustive_structs)]
pub struct PrettyOptions {
    pub target: bool,
    pub file: bool,
    pub line_number: bool,
}

impl Default for PrettyOptions {
    fn default() -> Self {
        Self {
            target: true,
            file: true,
            line_number: true,
        }
    }
}

#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(default)
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(clippy::exhaustive_structs)]
pub struct JsonOptions {
    pub target: bool,
    pub file: bool,
    pub line_number: bool,
    pub current_span: bool,
    pub span_list: bool,
    pub flatten_event: bool,
}

impl Default for JsonOptions {
    fn default() -> Self {
        Self {
            target: false,
            file: false,
            line_number: false,
            current_span: false,
            span_list: false,
            flatten_event: true,
        }
    }
}

#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "lowercase")
)]
#[derive(Clone)]
#[non_exhaustive]
#[allow(clippy::large_enum_variant)]
pub enum Format {
    Pretty(PrettyOptions),
    Compact(crate::Formatter),
    Json(JsonOptions),
    #[cfg_attr(feature = "serde", serde(skip))]
    Custom(Arc<dyn FormatEvent<Registry, DefaultFields> + Send + Sync>),
}

impl Format {
    #[must_use]
    pub fn custom(
        formatter: impl FormatEvent<Registry, DefaultFields> + Send + Sync + 'static,
    ) -> Self {
        Self::Custom(Arc::new(formatter))
    }
}

impl std::fmt::Debug for Format {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pretty(options) => f.debug_tuple("Pretty").field(options).finish(),
            Self::Compact(formatter) => f.debug_tuple("Compact").field(formatter).finish(),
            Self::Json(options) => f.debug_tuple("Json").field(options).finish(),
            Self::Custom(_) => f.debug_tuple("Custom").finish_non_exhaustive(),
        }
    }
}

impl Default for Format {
    fn default() -> Self {
        Self::Compact(crate::Formatter::new())
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
#[derive(Clone, Copy, Debug, Default)]
#[non_exhaustive]
pub enum Rotation {
    #[default]
    None,
    Rename,
    #[cfg(feature = "compress")]
    Compress,
}
#[cfg(feature = "file")]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[allow(clippy::module_name_repetitions)]
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct FileConfig {
    pub(crate) path: PathBuf,
    #[cfg_attr(feature = "serde", serde(default))]
    pub(crate) rotation: Rotation,
}

#[cfg(feature = "file")]
impl FileConfig {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            rotation: Rotation::default(),
        }
    }

    pub const fn with_rotation(mut self, rotation: Rotation) -> Self {
        self.rotation = rotation;
        self
    }
}

#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "lowercase")
)]
#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum WriterTarget {
    Stdout,
    Stderr,
    #[cfg_attr(feature = "serde", serde(skip))]
    Custom(Arc<BoxMakeWriter>),
    #[cfg(feature = "file")]
    File(FileConfig),
    #[cfg(acta_async)]
    AsyncStdout(AsyncMode),
    #[cfg(acta_async)]
    AsyncStderr(AsyncMode),
}

impl WriterTarget {
    #[must_use]
    pub fn custom(writer: impl for<'a> MakeWriter<'a> + Send + Sync + 'static) -> Self {
        Self::Custom(Arc::new(BoxMakeWriter::new(writer)))
    }
}

#[cfg(all(feature = "custom-async", feature = "serde"))]
const fn default_async_buffer_size() -> usize {
    4096
}

#[cfg(acta_async)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "lowercase")
)]
#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub enum AsyncMode {
    #[cfg(feature = "custom-async")]
    Custom {
        #[cfg_attr(feature = "serde", serde(default = "default_async_buffer_size"))]
        buffer_size: usize,
    },
    #[cfg(feature = "native-async")]
    Native,
}

#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(default)
)]
#[derive(Clone, Debug)]
pub struct Writer {
    pub(crate) format: Format,
    pub(crate) color_depth: Option<ColorDepth>,
    #[cfg_attr(feature = "serde", serde(with = "serde_adapters::levels"))]
    pub(crate) stacktrace: Vec<tracing::Level>,
    pub(crate) target: WriterTarget,
}

impl Default for Writer {
    fn default() -> Self {
        Self {
            format: Format::default(),
            color_depth: None,
            stacktrace: vec![tracing::Level::ERROR],
            target: WriterTarget::Stdout,
        }
    }
}

impl Writer {
    #[must_use]
    pub fn custom(writer: impl for<'a> MakeWriter<'a> + Send + Sync + 'static) -> Self {
        Self::default().with_target(WriterTarget::custom(writer))
    }

    #[must_use]
    pub fn stdout() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn stderr() -> Self {
        Self::default().with_target(WriterTarget::Stderr)
    }

    #[cfg(feature = "file")]
    #[must_use]
    pub fn file(path: impl Into<PathBuf>) -> Self {
        Self::default().with_target(WriterTarget::File(FileConfig::new(path)))
    }

    #[must_use]
    #[allow(clippy::missing_const_for_fn)]
    pub fn with_target(mut self, target: WriterTarget) -> Self {
        self.target = target;
        self
    }

    #[must_use]
    pub fn with_format(mut self, format: Format) -> Self {
        self.format = format;
        self
    }

    #[must_use]
    pub const fn with_color_depth(mut self, depth: ColorDepth) -> Self {
        self.color_depth = Some(depth);
        self
    }

    #[must_use]
    pub fn with_stacktrace(
        mut self,
        level: impl Into<tracing::level_filters::LevelFilter>,
    ) -> Self {
        if let Some(level) = level.into().into_level() {
            if !self.stacktrace.contains(&level) {
                self.stacktrace.push(level);
            }
        } else {
            self.stacktrace.clear();
        }
        self
    }
}

#[allow(clippy::single_call_fn)]
fn default_filter() -> tracing_subscriber::EnvFilter {
    tracing_subscriber::EnvFilter::new("info")
}

#[allow(clippy::single_call_fn)]
fn default_writers() -> Vec<Writer> {
    vec![Writer::default()]
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct Config {
    #[cfg_attr(
        feature = "serde",
        serde(default = "default_filter", with = "serde_adapters::filter")
    )]
    pub(crate) filter: tracing_subscriber::EnvFilter,
    #[cfg_attr(feature = "serde", serde(default = "default_writers"))]
    pub(crate) writers: Vec<Writer>,
}

impl Config {
    pub const fn builder() -> ConfigBuilder {
        ConfigBuilder {
            filter: None,
            writers: None,
        }
    }
}

#[derive(Debug)]
#[must_use]
#[allow(clippy::module_name_repetitions)]
pub struct ConfigBuilder {
    filter: Option<tracing_subscriber::EnvFilter>,
    writers: Option<Vec<Writer>>,
}

impl ConfigBuilder {
    pub fn with_filter(mut self, filter: tracing_subscriber::EnvFilter) -> Self {
        self.filter = Some(filter);
        self
    }

    pub fn with_writers(mut self, writers: impl IntoIterator<Item = Writer>) -> Self {
        self.writers = Some(writers.into_iter().collect());
        self
    }

    pub fn build(self) -> Config {
        Config {
            filter: self.filter.unwrap_or_else(default_filter),
            writers: self.writers.unwrap_or_else(default_writers),
        }
    }
}

#[cfg(feature = "serde")]
#[allow(clippy::single_call_fn)]
mod serde_adapters;

#[cfg(test)]
mod test;
