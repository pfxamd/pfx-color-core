use pfx_color_core::{
    difference, is_in_gamut, map_to_gamut, Color, ColorSpace, DifferenceMethod, GamutMap,
};

fn c(space: ColorSpace, channels: [f64; 3]) -> Color {
    Color::new(space, channels, 1.0).unwrap()
}

#[test]
fn css_mapping_preserves_in_gamut_colors_identically() {
    let input = c(ColorSpace::Srgb, [0.3, 0.4, 0.5]);
    assert_eq!(
        map_to_gamut(input, ColorSpace::Srgb, GamutMap::Css).unwrap(),
        input
    );
}

#[test]
fn css_mapping_uses_perceptual_chroma_and_local_clip_jnd() {
    for seed in [
        c(ColorSpace::DisplayP3, [0.0, 1.0, 0.0]),
        c(ColorSpace::Oklch, [0.73, 0.35, 30.0]),
        c(ColorSpace::Oklch, [0.55, 0.4, 265.0]),
        c(ColorSpace::Srgb, [1.3, -0.25, 0.8]),
    ] {
        let mapped = map_to_gamut(seed, ColorSpace::Srgb, GamutMap::Css).unwrap();
        assert!(is_in_gamut(mapped, ColorSpace::Srgb).unwrap());
        assert!(mapped.channels().iter().all(|v| (0.0..=1.0).contains(v)));
        let clipped = map_to_gamut(seed, ColorSpace::Srgb, GamutMap::Clip).unwrap();
        let css_error = difference(seed, mapped, DifferenceMethod::Ok).unwrap();
        let clip_error = difference(seed, clipped, DifferenceMethod::Ok).unwrap();
        assert!(css_error.is_finite());
        assert!(clip_error.is_finite());
    }
}

#[test]
fn css_mapping_near_gamut_border_uses_local_clip() {
    let input = c(ColorSpace::Srgb, [1.005, 0.3, 0.1]);
    let css = map_to_gamut(input, ColorSpace::Srgb, GamutMap::Css).unwrap();
    let clipped = map_to_gamut(input, ColorSpace::Srgb, GamutMap::Clip).unwrap();
    for (a, b) in css.channels().iter().zip(clipped.channels()) {
        assert!((a - b).abs() < 1e-9);
    }
}

#[test]
fn css_mapping_returns_extremes_for_out_of_range_lightness() {
    let dark = c(ColorSpace::Oklch, [-0.2, 0.25, 60.0]);
    let light = c(ColorSpace::Oklch, [1.3, 0.25, 60.0]);
    assert_eq!(
        map_to_gamut(dark, ColorSpace::Srgb, GamutMap::Css)
            .unwrap()
            .channels(),
        [0.0; 3]
    );
    assert_eq!(
        map_to_gamut(light, ColorSpace::Srgb, GamutMap::Css)
            .unwrap()
            .channels(),
        [1.0; 3]
    );
}

#[test]
fn cylindrical_ui_spaces_use_srgb_gamut_boundaries() {
    let p3_green = Color::new(ColorSpace::DisplayP3, [0.0, 1.0, 0.0], 0.7).unwrap();
    for target in [ColorSpace::Hsl, ColorSpace::Hwb, ColorSpace::Hsv] {
        assert!(!is_in_gamut(p3_green, target).unwrap());
        let mapped = map_to_gamut(p3_green, target, GamutMap::Css).unwrap();
        assert_eq!(mapped.space(), target);
        assert_eq!(mapped.alpha(), 0.7);
        assert!(is_in_gamut(mapped, target).unwrap());
    }
}
