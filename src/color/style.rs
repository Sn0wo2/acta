use anstyle_lossy::palette::Palette;
use anstyle_lossy::rgb_to_ansi;
use owo_colors::AnsiColors;
use owo_colors::Rgb;
use owo_colors::Style as OwoStyle;
use owo_colors::XtermColors;
use crate::ColorDepth;

const ANSI16_TABLE: [AnsiColors; 16] = [
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
];

#[derive(Clone, Copy, Debug)]
pub struct Styled {
    rgb: Rgb,
    depth: ColorDepth,
    on: bool,
}

impl Styled {
    pub(crate) const fn new(rgb: Rgb, depth: ColorDepth) -> Self {
        Self {
            rgb,
            depth,
            on: false,
        }
    }

    pub(crate) const fn dimmed(mut self) -> Self {
        self.rgb = Rgb(self.rgb.0 >> 2, self.rgb.1 >> 2, self.rgb.2 >> 2);
        self
    }

    pub(crate) const fn on(mut self) -> Self {
        self.on = true;
        self
    }

    pub(crate) const fn as_tuple(&self) -> (u8, u8, u8) {
        (self.rgb.0, self.rgb.1, self.rgb.2)
    }
}

impl From<Styled> for OwoStyle {
    fn from(s: Styled) -> Self {
        let style = Self::new();
        match s.depth {
            ColorDepth::TrueColor => {
                if s.on {
                    return style.on_truecolor(s.rgb.0, s.rgb.1, s.rgb.2);
                }
                style.truecolor(s.rgb.0, s.rgb.1, s.rgb.2)
            }
            ColorDepth::Ansi256 => {
                let ansi = XtermColors::from(ansi_colours::ansi256_from_rgb(s.as_tuple()));
                if s.on {
                    return style.on_color(ansi);
                }
                style.color(ansi)
            }
            ColorDepth::Ansi16 => {
                let idx = rgb_to_ansi(s.as_tuple().into(), Palette::default()) as usize;
                let ansi = ANSI16_TABLE.get(idx).copied().unwrap_or(AnsiColors::White);
                if s.on {
                    return style.on_color(ansi);
                }
                style.color(ansi)
            }
            ColorDepth::NoColor => style,
        }
    }
}
