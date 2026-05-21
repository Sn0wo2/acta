use ansi_colours::ansi256_from_rgb;

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
