use pfx_color_core::{Color, ColorError, ColorSpace};

const SPACES: [ColorSpace; 12] = [
    ColorSpace::Srgb,
    ColorSpace::SrgbLinear,
    ColorSpace::DisplayP3,
    ColorSpace::DisplayP3Linear,
    ColorSpace::Rec2020,
    ColorSpace::Rec2020Linear,
    ColorSpace::XyzD65,
    ColorSpace::XyzD50,
    ColorSpace::Lab,
    ColorSpace::Lch,
    ColorSpace::Oklab,
    ColorSpace::Oklch,
];

fn assert_near(actual: f64, expected: f64, tolerance: f64) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "expected {expected:.15}, received {actual:.15}, tolerance {tolerance}"
    );
}

fn check_channels(actual: [f64; 3], expected: [f64; 3], tolerance: f64) {
    for (a, e) in actual.into_iter().zip(expected) {
        assert_near(a, e, tolerance);
    }
}

#[test]
fn srgb_red_matches_w3c_xyz_d65_reference_matrix() {
    let red = Color::new(ColorSpace::Srgb, [1.0, 0.0, 0.0], 1.0).unwrap();
    let xyz = red.to(ColorSpace::XyzD65).unwrap();
    check_channels(
        xyz.channels(),
        [
            506752.0 / 1228815.0,
            87098.0 / 409605.0,
            7918.0 / 409605.0,
        ],
        1e-14,
    );
}

#[test]
fn display_p3_green_matches_w3c_xyz_reference_matrix() {
    let green = Color::new(ColorSpace::DisplayP3Linear, [0.0, 1.0, 0.0], 1.0).unwrap();
    check_channels(
        green.to(ColorSpace::XyzD65).unwrap().channels(),
        [189793.0 / 714400.0, 247089.0 / 357200.0, 32229.0 / 714400.0],
        1e-14,
    );
}

#[test]
fn css_rec2020_reference_matrix_is_implemented_exactly() {
    let red = Color::new(ColorSpace::Rec2020Linear, [1.0, 0.0, 0.0], 1.0).unwrap();
    check_channels(
        red.to(ColorSpace::XyzD65).unwrap().channels(),
        [
            63426534.0 / 99577255.0,
            26158966.0 / 99577255.0,
            0.0,
        ],
        1e-14,
    );
}

#[test]
fn lab_d50_white_is_reference_white() {
    let d50 = Color::new(
        ColorSpace::XyzD50,
        pfx_color_core::math::D50,
        1.0,
    )
    .unwrap();
    check_channels(d50.to(ColorSpace::Lab).unwrap().channels(), [100.0, 0.0, 0.0], 1e-12);
}

#[test]
fn oklab_matches_preexisting_pfx_reference() {
    let sample = Color::new(
        ColorSpace::Srgb,
        [0x33 as f64 / 255.0, 0x66 as f64 / 255.0, 0x99 as f64 / 255.0],
        1.0,
    )
    .unwrap();
    check_channels(
        sample.to(ColorSpace::Oklab).unwrap().channels(),
        [
            0.49931445584520834,
            -0.03304348760594705,
            -0.09296659206477714,
        ],
        2e-11,
    );
}

#[test]
fn all_supported_spaces_round_trip_without_rgb_quantization() {
    let source = Color::new(ColorSpace::Srgb, [0.17, 0.42, 0.85], 0.37).unwrap();

    for target in SPACES {
        let there = source.to(target).unwrap();
        assert_near(there.alpha(), 0.37, 0.0);
        let back = there.to(ColorSpace::Srgb).unwrap();
        check_channels(back.channels(), source.channels(), 2e-8);
        assert_near(back.alpha(), source.alpha(), 0.0);
    }
}

#[test]
fn converts_between_any_two_spaces_via_d65() {
    let source = Color::new(ColorSpace::DisplayP3, [0.24, 0.79, 0.38], 0.5).unwrap();
    for first in SPACES {
        for second in SPACES {
            let converted = source.to(first).unwrap().to(second).unwrap();
            let back = converted.to(ColorSpace::DisplayP3).unwrap();
            check_channels(back.channels(), source.channels(), 4e-8);
        }
    }
}

#[test]
fn polar_oklch_hue_is_expressed_in_degrees() {
    let color = Color::new(ColorSpace::Oklab, [0.6, 0.0, 0.2], 1.0).unwrap();
    let polar = color.to(ColorSpace::Oklch).unwrap();
    check_channels(polar.channels(), [0.6, 0.2, 90.0], 1e-11);
    check_channels(polar.to(ColorSpace::Oklab).unwrap().channels(), color.channels(), 1e-12);
}

#[test]
fn converted_out_of_gamut_channels_are_preserved() {
    let wide = Color::new(ColorSpace::DisplayP3, [0.0, 1.0, 0.0], 0.5).unwrap();
    let srgb = wide.to(ColorSpace::Srgb).unwrap();
    assert!(
        srgb.channels().iter().any(|v| *v < 0.0 || *v > 1.0),
        "wide-gamut values must not be clipped to display range"
    );
    let round_trip = srgb.to(ColorSpace::DisplayP3).unwrap();
    check_channels(round_trip.channels(), wide.channels(), 4e-8);
    assert_eq!(round_trip.alpha(), 0.5);
}

#[test]
fn extended_negative_rgb_round_trips_without_clipping() {
    let source = Color::new(ColorSpace::Srgb, [-0.25, 1.15, 0.45], 1.0).unwrap();
    let result = source
        .to(ColorSpace::SrgbLinear)
        .unwrap()
        .to(ColorSpace::Srgb)
        .unwrap();
    check_channels(result.channels(), source.channels(), 1e-12);
}

#[test]
fn repeated_identical_space_conversion_preserves_exact_input() {
    let source = Color::new(ColorSpace::Oklch, [0.6, 0.0, 273.0], 0.12).unwrap();
    assert_eq!(source.to(ColorSpace::Oklch).unwrap(), source);
}

#[test]
fn non_finite_conversion_result_is_reported_as_error() {
    let huge = Color::new(ColorSpace::Srgb, [f64::MAX, 0.0, 0.0], 1.0).unwrap();
    assert_eq!(huge.to(ColorSpace::XyzD65), Err(ColorError::NonFiniteResult));
}
