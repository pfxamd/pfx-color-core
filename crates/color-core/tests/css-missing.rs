use pfx_color_core::{
    format_css_missing, interpolate_css_missing, parse_css_missing,
    Color, ColorSpace, CssColor, HueMethod,
};
fn near(a: f64, b: f64) { assert!((a-b).abs() < 0.000001, "{a} vs {b}"); }

#[test]
fn parse_and_roundtrip_modern_missing_values_and_alpha() {
    for (text, mask) in [
        ("rgb(none 120 40 / none)", 9),
        ("hsl(120 none 30% / 0.5)", 2),
        ("hwb(none 20% 30% / none)", 9),
        ("lab(50 none none)", 6),
        ("lch(50 20 none)", 4),
        ("oklab(none 0.1 -0.1)", 1),
        ("oklch(none 0 none / none)", 13),
        ("color(display-p3 none 0.4 none / none)", 13),
    ] {
        let c = parse_css_missing(text).unwrap();
        assert_eq!(c.missing_mask(), mask, "{text}");
        let printed = format_css_missing(c).unwrap();
        assert_eq!(parse_css_missing(&printed).unwrap(), c, "{text} -> {printed}");
    }
    assert_eq!(parse_css_missing("#ff000080").unwrap().missing_mask(), 0);
}

#[test]
fn unsupported_legacy_or_advanced_grammar_is_rejected() {
    for text in [
        "rgba(none, 0, 0, 1)", "rgb(none, none, none)",
        "hsl(from red h s l)", "rgb(calc(2+3) none 0)",
        "rgb(var(--red) none 0)", "color(none 1 2 3)",
        "rgb(none 20 30 / none / 0.4)", "oklch(0.5 0 none 0)",
        "rgb(none nonsense 0)", "hsv(none 100 100)",
    ] { assert!(parse_css_missing(text).is_err(), "{text}"); }
}

#[test]
fn numeric_conversion_consumes_missing_as_zero_without_carrying_mask() {
    let source = parse_css_missing("rgb(none 128 64 / none)").unwrap();
    near(source.numeric().channels()[0], 0.0);
    near(source.numeric().alpha(), 0.0);
    let converted = source.convert_numeric(ColorSpace::Oklab).unwrap();
    assert_eq!(converted.space(), ColorSpace::Oklab);
    near(converted.alpha(), 0.0);
}

#[test]
fn same_space_analogous_components_and_both_missing() {
    let red = parse_css_missing("oklch(none 0.2 none / none)").unwrap();
    let blue = parse_css_missing("oklch(0.6 0.3 250 / 0.4)").unwrap();
    let mixed = interpolate_css_missing(red, blue, 0.5, ColorSpace::Oklch, HueMethod::Shorter).unwrap();
    assert_eq!(mixed.missing_mask(), 0);
    near(mixed.numeric().channels()[0], 0.6);
    near(mixed.numeric().channels()[2], 250.0);
    near(mixed.numeric().alpha(), 0.4);
    let all = parse_css_missing("oklch(none none none / none)").unwrap();
    let both = interpolate_css_missing(all, all, 0.5, ColorSpace::Oklch, HueMethod::Shorter).unwrap();
    assert_eq!(both.missing_mask(), 15);
}

#[test]
fn carried_sets_across_color_spaces() {
    let lab = parse_css_missing("lab(50 none none)").unwrap();
    let rgb = parse_css_missing("rgb(none none none / 50%)").unwrap();
    let neutral = CssColor::new(Color::new(ColorSpace::Lch, [60.0, 10.0, 30.0], 1.0).unwrap(), 0).unwrap();
    let out = interpolate_css_missing(lab, neutral, 0.0, ColorSpace::Lch, HueMethod::Shorter).unwrap();
    assert_eq!(out.missing_mask(), 0);
    let all = interpolate_css_missing(rgb, rgb, 0.5, ColorSpace::Oklab, HueMethod::Shorter).unwrap();
    assert_eq!(all.missing_mask(), 7);
    near(all.numeric().alpha(), 0.5);
}

#[test]
fn alpha_missing_does_not_equal_literal_zero_in_mixing() {
    let transparent = parse_css_missing("rgb(255 0 0 / 0)").unwrap();
    let unspecified = parse_css_missing("rgb(255 0 0 / none)").unwrap();
    let opaque = parse_css_missing("rgb(0 0 255 / 0.6)").unwrap();
    near(interpolate_css_missing(unspecified, opaque, 0.5, ColorSpace::Srgb, HueMethod::Shorter).unwrap().numeric().alpha(), 0.6);
    near(interpolate_css_missing(transparent, opaque, 0.5, ColorSpace::Srgb, HueMethod::Shorter).unwrap().numeric().alpha(), 0.3);
    let both = interpolate_css_missing(unspecified, unspecified, 0.5, ColorSpace::Srgb, HueMethod::Shorter).unwrap();
    assert_eq!(both.missing_mask() & 8, 8);
}
