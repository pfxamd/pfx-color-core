use pfx_color_core::{
    contrast_ratio, difference, interpolate, is_in_gamut, map_to_gamut, relative_luminance,
    Color, ColorError, ColorSpace, DifferenceMethod, GamutMap, HueMethod,
};

fn color(space: ColorSpace, coordinates: [f64; 3]) -> Color {
    Color::new(space, coordinates, 1.0).unwrap()
}

fn approx(actual: f64, expected: f64, tolerance: f64) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "expected {expected:.10}, got {actual:.10}"
    );
}

#[test]
fn cie76_and_ok_distance_are_euclidean_and_symmetric() {
    let a = color(ColorSpace::Lab, [50.0, 0.0, 0.0]);
    let b = color(ColorSpace::Lab, [53.0, 4.0, 0.0]);
    approx(difference(a, b, DifferenceMethod::Cie76).unwrap(), 5.0, 1e-12);
    approx(difference(b, a, DifferenceMethod::Cie76).unwrap(), 5.0, 1e-12);

    let x = color(ColorSpace::Oklab, [0.5, 0.0, 0.0]);
    let y = color(ColorSpace::Oklab, [0.5, 0.3, 0.4]);
    approx(difference(x, y, DifferenceMethod::Ok).unwrap(), 0.5, 1e-12);
    approx(difference(x, x, DifferenceMethod::Ok).unwrap(), 0.0, 0.0);
}

#[test]
fn ciede2000_matches_sharma_wu_dalal_reference_pairs() {
    // Reference supplementary pairs from Sharma et al., 2005.
    // Especially sensitive to the hue rotation term and wrapping at 360°.
    let pairs = [
        ([50.0, 2.6772, -79.7751], [50.0, 0.0, -82.7485], 2.0425),
        ([50.0, 3.1571, -77.2803], [50.0, 0.0, -82.7485], 2.8615),
        ([50.0, 2.8361, -74.0200], [50.0, 0.0, -82.7485], 3.4412),
        ([50.0, -1.3802, -84.2814], [50.0, 0.0, -82.7485], 1.0000),
        ([50.0, -1.1848, -84.8006], [50.0, 0.0, -82.7485], 1.0000),
        ([50.0, -0.9009, -85.5211], [50.0, 0.0, -82.7485], 1.0000),
    ];
    for (a, b, expected) in pairs {
        let left = color(ColorSpace::Lab, a);
        let right = color(ColorSpace::Lab, b);
        let forward = difference(left, right, DifferenceMethod::Ciede2000).unwrap();
        let reverse = difference(right, left, DifferenceMethod::Ciede2000).unwrap();
        approx(forward, expected, 0.00005);
        approx(reverse, forward, 1e-12);
    }
}

#[test]
fn wcag_black_white_and_self_contrast_are_reference_values() {
    let black = color(ColorSpace::Srgb, [0.0, 0.0, 0.0]);
    let white = color(ColorSpace::Srgb, [1.0, 1.0, 1.0]);
    approx(contrast_ratio(black, white).unwrap(), 21.0, 1e-12);
    approx(contrast_ratio(white, black).unwrap(), 21.0, 1e-12);
    approx(contrast_ratio(black, black).unwrap(), 1.0, 1e-12);
    approx(relative_luminance(black).unwrap(), 0.0, 0.0);
    approx(relative_luminance(white).unwrap(), 1.0, 1e-12);
}

#[test]
fn wcag_rejects_transparency_and_out_of_gamut_without_guesswork() {
    let white = color(ColorSpace::Srgb, [1.0, 1.0, 1.0]);
    let semi = Color::new(ColorSpace::Srgb, [0.0; 3], 0.5).unwrap();
    assert_eq!(contrast_ratio(semi, white), Err(ColorError::RequiresOpaque));
    let wide = color(ColorSpace::DisplayP3, [0.0, 1.0, 0.0]);
    assert_eq!(contrast_ratio(wide, white), Err(ColorError::OutOfGamut));
}

#[test]
fn rectangular_interpolation_is_linear_in_specified_space() {
    let red = color(ColorSpace::Srgb, [1.0, 0.0, 0.0]);
    let blue = color(ColorSpace::Srgb, [0.0, 0.0, 1.0]);
    let mid = interpolate(red, blue, 0.5, ColorSpace::Srgb, HueMethod::Shorter).unwrap();
    assert_eq!(mid.channels(), [0.5, 0.0, 0.5]);
    approx(mid.alpha(), 1.0, 0.0);
}

#[test]
fn interpolation_uses_premultiplied_alpha() {
    let transparent_red = Color::new(ColorSpace::Srgb, [1.0, 0.0, 0.0], 0.0).unwrap();
    let blue = color(ColorSpace::Srgb, [0.0, 0.0, 1.0]);
    let mid = interpolate(
        transparent_red,
        blue,
        0.5,
        ColorSpace::Srgb,
        HueMethod::Shorter,
    )
    .unwrap();
    assert_eq!(mid.channels(), [0.0, 0.0, 1.0]);
    approx(mid.alpha(), 0.5, 0.0);
    assert_eq!(
        interpolate(transparent_red, blue, 0.0, ColorSpace::Srgb, HueMethod::Shorter)
            .unwrap(),
        transparent_red
    );
}

#[test]
fn hue_arcs_and_powerless_hues_are_explicit() {
    let a = color(ColorSpace::Oklch, [0.6, 0.1, 350.0]);
    let b = color(ColorSpace::Oklch, [0.6, 0.1, 10.0]);
    let shorter = interpolate(a, b, 0.5, ColorSpace::Oklch, HueMethod::Shorter).unwrap();
    let longer = interpolate(a, b, 0.5, ColorSpace::Oklch, HueMethod::Longer).unwrap();
    let increasing = interpolate(a, b, 0.5, ColorSpace::Oklch, HueMethod::Increasing).unwrap();
    let decreasing = interpolate(a, b, 0.5, ColorSpace::Oklch, HueMethod::Decreasing).unwrap();
    approx(shorter.channels()[2], 0.0, 1e-12);
    approx(longer.channels()[2], 180.0, 1e-12);
    approx(increasing.channels()[2], 0.0, 1e-12);
    approx(decreasing.channels()[2], 180.0, 1e-12);

    let neutral = color(ColorSpace::Oklch, [0.6, 0.0, 0.0]);
    let tinted = color(ColorSpace::Oklch, [0.6, 0.2, 250.0]);
    let mixed = interpolate(neutral, tinted, 0.5, ColorSpace::Oklch, HueMethod::Shorter)
        .unwrap();
    approx(mixed.channels()[2], 250.0, 1e-12);
}

#[test]
fn interpolation_rejects_invalid_fraction() {
    let c = color(ColorSpace::Srgb, [0.1, 0.2, 0.3]);
    for fraction in [-0.1, 1.1, f64::NAN, f64::INFINITY] {
        assert_eq!(
            interpolate(c, c, fraction, ColorSpace::Oklab, HueMethod::Shorter),
            Err(ColorError::InvalidFraction)
        );
    }
}

#[test]
fn gamut_check_is_explicit_and_does_not_change_color() {
    let p3green = color(ColorSpace::DisplayP3, [0.0, 1.0, 0.0]);
    assert!(is_in_gamut(p3green, ColorSpace::DisplayP3).unwrap());
    assert!(!is_in_gamut(p3green, ColorSpace::Srgb).unwrap());
    assert!(is_in_gamut(p3green, ColorSpace::Oklch).unwrap());
    assert_eq!(p3green.channels(), [0.0, 1.0, 0.0]);
}

#[test]
fn gamut_mapping_results_are_bounded_and_preserve_alpha() {
    let wide = Color::new(ColorSpace::DisplayP3, [0.0, 1.0, 0.0], 0.65).unwrap();
    for method in [GamutMap::Clip, GamutMap::OklchChroma] {
        let mapped = map_to_gamut(wide, ColorSpace::Srgb, method).unwrap();
        assert!(is_in_gamut(mapped, ColorSpace::Srgb).unwrap());
        assert!(mapped.channels().iter().all(|v| (0.0..=1.0).contains(v)));
        approx(mapped.alpha(), 0.65, 0.0);
        assert_eq!(mapped.space(), ColorSpace::Srgb);
    }
    let already = color(ColorSpace::Srgb, [0.2, 0.3, 0.4]);
    assert_eq!(
        map_to_gamut(already, ColorSpace::Srgb, GamutMap::OklchChroma).unwrap(),
        already
    );
}

#[test]
fn chroma_mapping_retains_approximate_oklab_lightness() {
    let wide = color(ColorSpace::DisplayP3, [0.0, 1.0, 0.0]);
    let mapped = map_to_gamut(wide, ColorSpace::Srgb, GamutMap::OklchChroma).unwrap();
    let source_ok = wide.to(ColorSpace::Oklab).unwrap().channels();
    let mapped_ok = mapped.to(ColorSpace::Oklab).unwrap().channels();
    approx(source_ok[0], mapped_ok[0], 1e-5);
}

#[test]
fn mapped_black_and_white_outside_lightness_are_bounded() {
    let dark = color(ColorSpace::Oklch, [-0.2, 0.5, 80.0]);
    let light = color(ColorSpace::Oklch, [1.4, 0.5, 80.0]);
    let black = map_to_gamut(dark, ColorSpace::Srgb, GamutMap::OklchChroma).unwrap();
    let white = map_to_gamut(light, ColorSpace::Srgb, GamutMap::OklchChroma).unwrap();
    assert_eq!(black.channels(), [0.0, 0.0, 0.0]);
    assert_eq!(white.channels(), [1.0, 1.0, 1.0]);
}
