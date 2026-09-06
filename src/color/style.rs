use crate::ColorDepth;
use anstyle_lossy::palette::Palette;
use anstyle_lossy::rgb_to_ansi;
use owo_colors::Style as OwoStyle;
use owo_colors::XtermColors;
use owo_colors::{AnsiColors, DynColors};

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

pub(crate) fn rgb_to_owo((r, g, b): (u8, u8, u8), depth: ColorDepth, background: bool) -> OwoStyle {
    let style = OwoStyle::new();
    let color = match depth {
        ColorDepth::TrueColor => Some(DynColors::Rgb(r, g, b)),
        ColorDepth::Ansi256 => Some(DynColors::Xterm(XtermColors::from(
            ansi_colours::ansi256_from_rgb((r, g, b)),
        ))),
        ColorDepth::Ansi16 => Some(DynColors::Ansi(
            ANSI16_TABLE
                .get(rgb_to_ansi((r, g, b).into(), Palette::default()) as usize)
                .copied()
                .unwrap_or(AnsiColors::White),
        )),
        ColorDepth::NoColor => None,
    };

    match color {
        Some(color) if background => style.on_color(color),
        Some(color) => style.color(color),
        None => style,
    }
}
