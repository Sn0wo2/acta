use ansi_colours::ansi256_from_rgb;

use super::style::rgb_to_owo;
use crate::ColorDepth;

#[test]
fn rgb256_primary_red() {
    assert_eq!(ansi256_from_rgb((255, 0, 0)), 196);
}
#[test]
fn rgb256_primary_green() {
    assert_eq!(ansi256_from_rgb((0, 255, 0)), 46);
}
#[test]
fn rgb256_primary_blue() {
    assert_eq!(ansi256_from_rgb((0, 0, 255)), 21);
}
#[test]
fn rgb256_gray() {
    assert_eq!(ansi256_from_rgb((128, 128, 128)), 244);
}
#[test]
fn rgb256_black() {
    assert_eq!(ansi256_from_rgb((0, 0, 0)), 16);
}
#[test]
fn rgb256_white() {
    assert_eq!(ansi256_from_rgb((255, 255, 255)), 231);
}

#[test]
fn rgb_styles_cover_color_depths_and_backgrounds() {
    for depth in [
        ColorDepth::TrueColor,
        ColorDepth::Ansi256,
        ColorDepth::Ansi16,
    ] {
        assert_ne!(
            format!("{}", rgb_to_owo((255, 0, 0), depth, false).style("x")),
            "x"
        );
        assert_ne!(
            format!("{}", rgb_to_owo((255, 0, 0), depth, true).style("x")),
            "x"
        );
    }

    assert_eq!(
        format!(
            "{}",
            rgb_to_owo((255, 0, 0), ColorDepth::NoColor, false).style("x")
        ),
        "x"
    );
    assert_eq!(
        format!(
            "{}",
            rgb_to_owo((255, 0, 0), ColorDepth::NoColor, true).style("x")
        ),
        "x"
    );
}
