#[cfg(feature = "nerd")]
use nerd_font_symbols::{cod, fa, ple};
use std::collections::HashMap;
#[cfg(feature = "file")]
use std::path::PathBuf;

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ColorDepth {
    TrueColor,
    Ansi256,
    Ansi16,
    NoColor,
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

    pub const DEFAULT: Self = Self {
        error: "ERROR",
        warn: " WARN",
        info: " INFO",
        debug: "DEBUG",
        trace: "TRACE",
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
    pub span_delimiter: &'static str,
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
        span_delimiter: &'static str,
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
            span_delimiter,
            span_join,
        }
    }

    pub const UNICODE: Self = Self {
        name: "unicode",
        #[rustfmt::skip] // 让下面两个括号可以对齐
        bracket_open:  "[",
        bracket_close: "]",
        #[rustfmt::skip] // ↑
        time_bracket_open:  "｢",
        time_bracket_close: "｣",
        separator: "┇", // \u{2507}
        arrow: ">",
        span_delimiter: "->",
        span_join: "»", // \u{00bb}
    };

    #[cfg(feature = "nerd")]
    pub const NERD: Self = Self {
        name: "nerd",
        bracket_open: ple::PLE_LEFT_HALF_CIRCLE_THICK,
        bracket_close: ple::PLE_RIGHT_HALF_CIRCLE_THICK,
        time_bracket_open: ple::PLE_LEFT_HALF_CIRCLE_THIN,
        time_bracket_close: ple::PLE_RIGHT_HALF_CIRCLE_THIN,
        separator: "┇", // \u{2507}
        arrow: fa::FA_CARET_RIGHT,
        span_delimiter: cod::COD_EXPORT,
        span_join: fa::FA_ANGLES_RIGHT,
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

#[derive(Clone, Copy, Debug, Default)]
#[allow(clippy::exhaustive_structs)]
pub struct Style {
    pub theme: Theme,
    pub icons: Icons,
    pub labels: LevelLabels,
}

#[allow(clippy::module_name_repetitions)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct LayerConfig {
    #[cfg_attr(feature = "serde", serde(default))]
    pub target: bool,
    #[cfg_attr(feature = "serde", serde(default))]
    pub file: bool,
    #[cfg_attr(feature = "serde", serde(default))]
    pub line_number: bool,
    #[cfg_attr(feature = "serde", serde(default))]
    pub current_span: bool,
    #[cfg_attr(feature = "serde", serde(default))]
    pub span_list: bool,
    /// Flatten event into a single line (Json only)
    #[cfg_attr(feature = "serde", serde(default))]
    pub flatten_event: bool,
    #[cfg_attr(feature = "serde", serde(default))]
    pub thread_ids: bool,
    #[cfg_attr(feature = "serde", serde(default))]
    pub thread_names: bool,
}

impl LayerConfig {
    pub const fn pretty() -> Self {
        Self {
            target: true,
            file: true,
            line_number: true,
            current_span: false,
            span_list: false,
            flatten_event: false,
            thread_ids: false,
            thread_names: false,
        }
    }

    pub const fn compact() -> Self {
        Self {
            target: false,
            file: false,
            line_number: false,
            current_span: false,
            span_list: false,
            flatten_event: false,
            thread_ids: false,
            thread_names: false,
        }
    }

    pub const fn json() -> Self {
        Self {
            target: false,
            file: false,
            line_number: false,
            current_span: false,
            span_list: false,
            flatten_event: true,
            thread_ids: false,
            thread_names: false,
        }
    }
}

impl Default for LayerConfig {
    fn default() -> Self {
        Self::compact()
    }
}

#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "lowercase")
)]
#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum Format {
    Pretty(LayerConfig),
    Compact(LayerConfig),
    Json(LayerConfig),
}

impl Default for Format {
    fn default() -> Self {
        Self::Compact(LayerConfig::compact())
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

#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "lowercase")
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Level {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
    Off,
}

impl Level {
    pub const fn as_directive(self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warn => "warn",
            Self::Info => "info",
            Self::Debug => "debug",
            Self::Trace => "trace",
            Self::Off => "off",
        }
    }
}

/// Tracing filter directive.
///
/// Built either from a `Level` (structured) or a raw `EnvFilter`-compatible
/// directive string. Per-target overrides added via [`with_target`] are
/// appended after the base; [`remove_target`] only removes entries that were
/// added structurally — entries embedded in a raw base string cannot be
/// removed without rebuilding the filter.
///
/// [`with_target`]: Filter::with_target
/// [`remove_target`]: Filter::remove_target
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct Filter {
    base: compact_str::CompactString,
    targets: HashMap<compact_str::CompactString, Level>,
}

impl Filter {
    pub fn new(level: Level) -> Self {
        Self {
            base: level.as_directive().into(),
            targets: HashMap::new(),
        }
    }

    /// Build a `Filter` from a raw `EnvFilter`-style directive string,
    /// e.g. `"info,my_crate=debug,my_crate::db=trace"`.
    pub fn from_directive(directive: impl Into<compact_str::CompactString>) -> Self {
        Self {
            base: directive.into(),
            targets: HashMap::new(),
        }
    }

    pub fn with_target(
        &mut self,
        target: impl Into<compact_str::CompactString>,
        level: Level,
    ) -> &mut Self {
        self.targets.insert(target.into(), level);
        self
    }

    pub fn remove_target(&mut self, target: &str) -> bool {
        self.targets.remove(target).is_some()
    }

    pub fn as_directive(&self) -> String {
        let mut directive = String::from(self.base.as_str());
        for (target, level) in &self.targets {
            directive.push(',');
            directive.push_str(target);
            directive.push('=');
            directive.push_str(level.as_directive());
        }
        directive
    }
}

impl Default for Filter {
    fn default() -> Self {
        Self::new(Level::Info)
    }
}

impl From<Level> for Filter {
    fn from(level: Level) -> Self {
        Self::new(level)
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
    #[cfg(feature = "file")]
    File {
        path: PathBuf,
        #[cfg_attr(feature = "serde", serde(default))]
        rotation: Rotation,
    },
    #[cfg(any(feature = "custom-async", feature = "native-async"))]
    AsyncStdout(AsyncMode),
    #[cfg(any(feature = "custom-async", feature = "native-async"))]
    AsyncStderr(AsyncMode),
}

#[cfg(any(feature = "custom-async", feature = "native-async"))]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "lowercase")
)]
#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub enum AsyncMode {
    #[cfg(feature = "custom-async")]
    Custom,
    #[cfg(feature = "native-async")]
    Native,
}

#[cfg(any(feature = "custom-async", feature = "native-async"))]
#[allow(clippy::derivable_impls)]
impl Default for AsyncMode {
    fn default() -> Self {
        #[cfg(feature = "custom-async")]
        return Self::Custom;
        #[cfg(all(feature = "native-async", not(feature = "custom-async")))]
        return Self::Native;
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[allow(clippy::exhaustive_structs)]
#[derive(Clone, Debug)]
pub struct Writer {
    pub format: Format,
    #[cfg_attr(feature = "serde", serde(default))]
    pub ansi: bool,
    #[cfg_attr(feature = "serde", serde(default))]
    pub color_depth: Option<ColorDepth>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub show_path: bool,
    #[cfg_attr(feature = "serde", serde(default))]
    pub show_spans: bool,
    #[cfg_attr(feature = "serde", serde(default))]
    pub time_format: Option<String>,
    #[cfg_attr(feature = "serde", serde(skip))]
    pub style: Style,
    pub target: WriterTarget,
}

impl Default for Writer {
    fn default() -> Self {
        Self {
            format: Format::default(),
            ansi: true,
            color_depth: None,
            show_path: true,
            show_spans: true,
            time_format: None,
            style: Style::default(),
            target: WriterTarget::Stdout,
        }
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct Config {
    #[cfg_attr(feature = "serde", serde(default))]
    pub filter: Filter,
    #[cfg_attr(feature = "serde", serde(default))]
    pub writers: Vec<Writer>,
}

impl Config {
    pub fn builder() -> ConfigBuilder {
        ConfigBuilder::default()
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            filter: Filter::default(),
            writers: vec![Writer::default()],
        }
    }
}

#[derive(Default, Debug)]
#[must_use]
#[allow(clippy::module_name_repetitions)]
pub struct ConfigBuilder {
    filter: Option<Filter>,
    writers: Vec<Writer>,
}

impl ConfigBuilder {
    /// Convenience: set the filter to a single level.
    pub fn level(mut self, level: Level) -> Self {
        self.filter = Some(Filter::new(level));
        self
    }

    /// Set the full filter (for raw directives or pre-built filters).
    pub fn filter(mut self, filter: Filter) -> Self {
        self.filter = Some(filter);
        self
    }

    pub fn with_writer(mut self, writer: Writer) -> Self {
        self.writers.push(writer);
        self
    }

    pub fn build(self) -> Config {
        let defaults = Config::default();
        Config {
            filter: self.filter.unwrap_or(defaults.filter),
            writers: if self.writers.is_empty() {
                defaults.writers
            } else {
                self.writers
            },
        }
    }
}

impl From<ConfigBuilder> for Config {
    fn from(b: ConfigBuilder) -> Self {
        b.build()
    }
}

#[cfg(test)]
mod test;
