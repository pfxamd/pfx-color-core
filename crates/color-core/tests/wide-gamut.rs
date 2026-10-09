//! Reference cases for Adobe RGB (1998) and Kodak ProPhoto RGB.
//! Equations and matrices: W3C CSS Color 4, 2026-10-08.

use pfx_color_core::{format_css, is_in_gamut, parse_css, Color, ColorSpace};

fn color(space: ColorSpace, channels: [f64; 3]) -> Color {
    Color::new(space, channels, 1.0).unwrap()
}
fn near(actual: f64, expected: f64, tolerance: f64) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "expected {expected}, actual {actual}, tolerance {tolerance}"
    );
}
fn near_channels(actual: [f64; 3], expected: [f64; 3], tolerance: f64) {
    for (value, target) in actual.into_iter().zip(expected) {
        near(value, target, tolerance);
    }
}

#[test]
fn adobe_rgb_red_xyz_matches_w3c_matrix() {
    let red = color(ColorSpace::A98Rgb, [1.0, 0.0, 0.0]);
    near_channels(
        red.to(ColorSpace::XyzD65).unwrap().channels(),
        [
            573536.0 / 994567.0,
            591459.0 / 1989134.0,
            53769.0 / 1989134.0,
        ],
        1e-14,
    );
}

#[test]
fn prophoto_red_xyz_d50_matches_w3c_matrix() {
    let red = color(ColorSpace::ProPhotoRgb, [1.0, 0.0, 0.0]);
    near_channels(
        red.to(ColorSpace::XyzD50).unwrap().channels(),
        [0.797_766_644_900_642_3, 0.288_074_828_819_401_3, 0.0],
        1e-13,
    );
}

#[test]
fn adobe_prophoto_extended_channels_round_trip() {
    let spaces = [ColorSpace::A98Rgb, ColorSpace::ProPhotoRgb];
    for space in spaces {
        for input in [
            [-0.15, 0.2, 1.25],
            [0.0, 0.01, 0.03125],
            [0.15, 0.6, 0.7],
            [0.9, 0.3, 0.1],
        ] {
            let source = color(space, input);
            let result = source.to(ColorSpace::XyzD65).unwrap().to(space).unwrap();
            near_channels(result.channels(), input, 2e-8);
        }
    }
}

#[test]
fn prophoto_linear_to_gamma_to_linear_round_trip_near_toe() {
    for encoded in [-0.08, -0.03125, -0.005, 0.0, 0.005, 0.03125, 0.3] {
        let source = color(ColorSpace::ProPhotoRgb, [encoded, 0.5, 0.7]);
        let back = source
            .to(ColorSpace::XyzD50)
            .unwrap()
            .to(ColorSpace::ProPhotoRgb)
            .unwrap();
        near(back.channels()[0], encoded, 2e-9);
    }
}

#[test]
fn css_parses_formats_both_spaces_without_clipping() {
    for (name, space) in [
        ("a98-rgb", ColorSpace::A98Rgb),
        ("prophoto-rgb", ColorSpace::ProPhotoRgb),
    ] {
        let source = format!("color({name} 0.14 0.56 0.8 / 0.45)");
        let parsed = parse_css(&source).unwrap();
        assert_eq!(parsed.space(), space);
        near(parsed.alpha(), 0.45, 0.0);
        assert!(is_in_gamut(parsed, space).unwrap());
        let formatted = format_css(parsed).unwrap();
        assert!(formatted.starts_with(&format!("color({name} ")));
        let decoded = parse_css(&formatted).unwrap();
        near_channels(decoded.channels(), parsed.channels(), 1e-15);
        near(decoded.alpha(), parsed.alpha(), 1e-15);
    }
}

#[test]
fn gamut_considers_a98_and_prophoto_bounded_not_xyz() {
    for space in [ColorSpace::A98Rgb, ColorSpace::ProPhotoRgb] {
        let extended = color(space, [1.2, 0.1, 0.5]);
        assert!(!is_in_gamut(extended, space).unwrap());
        assert!(is_in_gamut(extended, ColorSpace::XyzD65).unwrap());
    }
}
