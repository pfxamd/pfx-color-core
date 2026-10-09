use pfx_color_core::{Color, ColorSpace};
use pfx_color_core::math::D65;

fn near(actual: f64, expected: f64, epsilon: f64) {
    assert!((actual - expected).abs() <= epsilon, "actual {actual}, expected {expected}");
}
#[test]
fn lab_d65_white_and_black_are_reference_lightness() {
    let white = Color::new(ColorSpace::XyzD65, D65, 1.0).unwrap();
    let lab = white.to(ColorSpace::LabD65).unwrap();
    near(lab.channels()[0], 100.0, 1e-12);
    near(lab.channels()[1], 0.0, 1e-12);
    near(lab.channels()[2], 0.0, 1e-12);
    let black = Color::new(ColorSpace::Srgb, [0.0; 3], 1.0).unwrap();
    let lab = black.to(ColorSpace::LabD65).unwrap();
    near(lab.channels()[0], 0.0, 1e-12);
}
#[test]
fn lab_d65_round_trip_preserves_extended_srgb() {
    for input in [[0.2, 0.3, 0.4], [0.9, 0.7, 0.1], [-0.15, 0.2, 1.25]] {
        let color = Color::new(ColorSpace::Srgb, input, 0.6).unwrap();
        let returned = color.to(ColorSpace::LabD65).unwrap().to(ColorSpace::Srgb).unwrap();
        for (a,b) in returned.channels().into_iter().zip(input) {
            near(a, b, 5e-8);
        }
        near(returned.alpha(), 0.6, 0.0);
    }
}
