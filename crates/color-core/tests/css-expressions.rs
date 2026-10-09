use pfx_color_core::{parse_css_missing, parse_css, ColorSpace, ColorError};

fn near(actual: f64, expected: f64) {
    assert!((actual - expected).abs() < 0.000001, "{actual} vs {expected}");
}

#[test]
fn relative_rgb_reference_channels_and_inherited_alpha() {
    let red = parse_css_missing("rgb(from red r g b)").unwrap();
    assert_eq!(red.space(), ColorSpace::Srgb);
    near(red.numeric().channels()[0], 1.0);
    near(red.numeric().channels()[1], 0.0);
    near(red.numeric().alpha(), 1.0);

    let magenta = parse_css_missing("rgb(from rgb(100 25 0 / 0.7) r g calc(b + 255))").unwrap();
    let channels = magenta.numeric().channels();
    near(channels[0], 100.0 / 255.0);
    near(channels[1], 25.0 / 255.0);
    near(channels[2], 1.0);
    near(magenta.numeric().alpha(), 0.7);
}

#[test]
fn relative_hsl_units_and_math_chains() {
    let color = parse_css_missing(
        "hsl(from red calc(h + 60) calc(s - 20) calc(l + 10) / calc(alpha - 0.3))",
    ).unwrap();
    assert_eq!(color.space(), ColorSpace::Hsl);
    let channels = color.numeric().channels();
    near(channels[0], 60.0);
    near(channels[1], 80.0);
    near(channels[2], 60.0);
    near(color.numeric().alpha(), 0.7);
}

#[test]
fn relative_color_space_identifiers_and_no_automatic_clipping() {
    let color = parse_css_missing(
        "color(from hsl(0 100% 50%) srgb calc(r - 0.4) calc(g + 0.1) calc(b + 0.6) / calc(alpha - 0.1))",
    ).unwrap();
    assert_eq!(color.space(), ColorSpace::Srgb);
    for (actual, expected) in color.numeric().channels().iter().zip([0.6, 0.1, 0.6]) {
        near(*actual, expected);
    }
    near(color.numeric().alpha(), 0.9);
    let wide = parse_css_missing(
        "rgb(from color(display-p3 1 0.5 0.5) calc(r - 1) g b)",
    ).unwrap();
    assert!(wide.numeric().channels()[0] > 1.0);
}

#[test]
fn absolute_math_with_typed_percentage_angle_and_nested_parentheses() {
    let rgb = parse_css_missing("rgb(calc((20 + 30) * 2) 0 calc(25% * 2))").unwrap();
    near(rgb.numeric().channels()[0], 100.0 / 255.0);
    near(rgb.numeric().channels()[2], 0.5);
    let hsl = parse_css_missing("hsl(calc(0.25turn + 30deg) calc(50% + 25%) 40%)").unwrap();
    near(hsl.numeric().channels()[0], 120.0);
    near(hsl.numeric().channels()[1], 75.0);
    let alpha = parse_css_missing("rgb(255 0 0 / calc((1 - 0.25) / 3))").unwrap();
    near(alpha.numeric().alpha(), 0.25);
}

#[test]
fn nested_relative_origin_and_none_preservation() {
    let nested = parse_css_missing(
        "rgb(from hsl(from red h s l / alpha) r g b / alpha)",
    ).unwrap();
    near(nested.numeric().channels()[0], 1.0);
    near(nested.numeric().channels()[2], 0.0);
    let missing = parse_css_missing("rgb(from red none g calc(b + 10) / none)").unwrap();
    assert_eq!(missing.missing_mask(), 9);
    near(missing.numeric().channels()[2], 10.0 / 255.0);
    near(missing.numeric().alpha(), 0.0);
    let copy = parse_css_missing("oklch(from #ff0000 l c calc(h + 120))").unwrap();
    assert_eq!(copy.space(), ColorSpace::Oklch);
    let original = parse_css("#ff0000").unwrap().to(ColorSpace::Oklch).unwrap();
    near(copy.numeric().channels()[2], (original.channels()[2] + 120.0) % 360.0);
}

#[test]
fn grammar_validation_rejects_mixed_math_units_external_variables_and_division_errors() {
    for value in [
        "rgb(calc(20% + 40) 0 0)",
        "hsl(calc(20deg + 1%) 80% 50%)",
        "rgb(from red calc(r + 10%) g b)",
        "rgb(from red calc(r / 0) g b)",
        "rgb(var(--r) 10 20)",
        "rgb(from var(--origin) r g b)",
        "rgb(calc(2 * 10% * 20%) 0 0)",
        "rgb(calc(1px + 2px) 0 0)",
        "hsl(calc(1e999) 20% 40%)",
        "rgb(from #f00 calc(r + nope) g b)",
        "rgb(calc(1 + ) 0 0)",
        "rgb(from red r g b / calc(alpha + 20%))",
    ] {
        assert!(parse_css_missing(value).is_err(), "{value}");
    }
    assert!(matches!(
        parse_css_missing("rgb(calc(10% + 20) 0 0)"),
        Err(ColorError::InvalidSyntax)
    ));
}

#[test]
fn depth_limit_and_legacy_numeric_api_remain_separate() {
    let mut expression = String::from("1");
    for _ in 0..20 {
        expression = format!("calc({expression})");
    }
    assert!(parse_css_missing(&format!("rgb({expression} 0 0)")).is_err());
    assert!(parse_css("rgb(from red r g b)").is_err());
    assert!(parse_css("rgb(calc(10 + 20) 0 0)").is_err());
    let standard = parse_css_missing("rgb(none 50 10 / none)").unwrap();
    assert_eq!(standard.missing_mask(), 9);
}
