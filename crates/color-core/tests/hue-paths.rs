use pfx_color_core::{interpolate, Color, ColorSpace, HueMethod};
fn c(space: ColorSpace, channels: [f64; 3], alpha: f64) -> Color {
    Color::new(space, channels, alpha).unwrap()
}
fn near(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() < 1e-9,
        "expected {expected}, got {actual}"
    );
}
#[test]
fn raw_hue_preserves_direct_path_in_oklch() {
    let first = c(ColorSpace::Oklch, [0.6, 0.1, 350.0], 1.0);
    let last = c(ColorSpace::Oklch, [0.6, 0.1, 10.0], 1.0);
    near(
        interpolate(first, last, 0.5, ColorSpace::Oklch, HueMethod::Raw)
            .unwrap()
            .channels()[2],
        180.0,
    );
    near(
        interpolate(first, last, 0.5, ColorSpace::Oklch, HueMethod::Shorter)
            .unwrap()
            .channels()[2],
        0.0,
    );
}
#[test]
fn hsl_hwb_hsv_apply_hue_paths_without_premultiplication() {
    for space in [ColorSpace::Hsl, ColorSpace::Hwb, ColorSpace::Hsv] {
        let from = c(space, [350.0, 40.0, 40.0], 0.2);
        let to = c(space, [10.0, 70.0, 50.0], 0.8);
        let short = interpolate(from, to, 0.5, space, HueMethod::Shorter).unwrap();
        let raw = interpolate(from, to, 0.5, space, HueMethod::Raw).unwrap();
        near(short.channels()[0], 0.0);
        near(raw.channels()[0], 180.0);
        near(short.alpha(), 0.5);
        near(raw.alpha(), 0.5);
        near(raw.channels()[1], 64.0);
    }
}
#[test]
fn powerless_hue_uses_chromatic_endpoint_in_hsl_hwb_hsv() {
    for (space, neutral) in [
        (ColorSpace::Hsl, [290.0, 0.0, 50.0]),
        (ColorSpace::Hsv, [290.0, 0.0, 50.0]),
        (ColorSpace::Hwb, [290.0, 70.0, 40.0]),
    ] {
        let neutral = c(space, neutral, 1.0);
        let color = c(space, [30.0, 60.0, 40.0], 1.0);
        let mixed = interpolate(neutral, color, 0.5, space, HueMethod::Shorter).unwrap();
        near(mixed.channels()[0], 30.0);
    }
}
#[test]
fn raw_hue_preserves_unwrapped_coordinates() {
    let first = c(ColorSpace::Hsl, [350.0, 100.0, 50.0], 1.0);
    let last = c(ColorSpace::Hsl, [370.0, 100.0, 50.0], 1.0);
    near(
        interpolate(first, last, 0.5, ColorSpace::Hsl, HueMethod::Raw)
            .unwrap()
            .channels()[0],
        360.0,
    );
}
