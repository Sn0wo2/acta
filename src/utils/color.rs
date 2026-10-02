use crate::ColorDepth;
use anstyle_lossy::palette::Palette;
use anstyle_lossy::rgb_to_ansi;
use owo_colors::XtermColors;
use owo_colors::{AnsiColors, DynColors};

pub(crate) fn themed((r, g, b): (u8, u8, u8), depth: ColorDepth) -> DynColors {
    match depth {
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
            .get(rgb_to_ansi((r, g, b).into(), Palette::default()) as usize)
            .copied()
            .unwrap_or(AnsiColors::White),
        ),
        ColorDepth::NoColor => DynColors::Ansi(AnsiColors::Default),
    }
}
