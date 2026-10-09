use pfx_color_core::{format_css, format_hex, parse_css, Color, ColorError, ColorSpace, GamutMap};

fn near(a: f64, b: f64, e: f64) {
    assert!((a - b).abs() <= e, "expected {b}, actual {a}");
}
fn rgb(c: Color, expected: [f64; 3]) {
    let actual = c.to(ColorSpace::Srgb).unwrap().channels();
    for i in 0..3 {
        near(actual[i], expected[i], 1e-10);
    }
}

#[test]
fn hex_short_long_and_alpha_are_specified() {
    for (source, rgb_value, alpha) in [
        ("#f06", [1.0, 0.0, 0.4], 1.0),
        ("#ff0066", [1.0, 0.0, 0.4], 1.0),
        ("#f068", [1.0, 0.0, 0.4], 136.0 / 255.0),
        ("#ff006688", [1.0, 0.0, 0.4], 136.0 / 255.0),
        ("#0000", [0.0, 0.0, 0.0], 0.0),
    ] {
        let c = parse_css(source).unwrap();
        rgb(c, rgb_value);
        near(c.alpha(), alpha, 0.0);
        assert_eq!(
            format_hex(c, GamutMap::Clip).unwrap().len(),
            if alpha == 1.0 { 7 } else { 9 }
        );
    }
    assert_eq!(
        format_hex(parse_css("#f06").unwrap(), GamutMap::Clip).unwrap(),
        "#ff0066"
    );
    assert_eq!(
        format_hex(parse_css("#f068").unwrap(), GamutMap::Clip).unwrap(),
        "#ff006688"
    );
}

#[test]
fn rgb_legacy_modern_percent_and_alpha() {
    for input in [
        "rgb(255 0 0 / 50%)",
        "rgba(255, 0, 0, 0.5)",
        "rgb(100% 0% 0% / .5)",
        "rgba(100%,0%,0%,50%)",
        "rgb(255 0% 0 / 50%)",
    ] {
        let c = parse_css(input).unwrap();
        rgb(c, [1.0, 0.0, 0.0]);
        near(c.alpha(), 0.5, 0.0);
    }
    rgb(
        parse_css("rgb(300 -1 0)").unwrap(),
        [300.0 / 255.0, -1.0 / 255.0, 0.0],
    );
    assert_eq!(
        parse_css("rgba(255, 0%, 0, 0.5)"),
        Err(ColorError::InvalidSyntax)
    );
}

#[test]
fn hsl_hwb_hue_units_and_conversion() {
    for source in [
        "hsl(120 100% 50%)",
        "hsl(1/3turn 100% 50%)", // accepted? invalid CSS math; checked separately below
    ]
    .iter()
    .take(1)
    {
        rgb(parse_css(source).unwrap(), [0.0, 1.0, 0.0]);
    }
    for source in [
        "hsl(120deg 100% 50%)",
        "hsla(120, 100%, 50%, 1)",
        "hsl(0.3333333333333333turn 100% 50%)",
    ] {
        let c = parse_css(source).unwrap();
        rgb(c, [0.0, 1.0, 0.0]);
    }
    rgb(parse_css("hwb(240 0% 0%)").unwrap(), [0.0, 0.0, 1.0]);
    rgb(parse_css("hwb(60 60% 60%)").unwrap(), [0.5, 0.5, 0.5]);
    assert_eq!(
        parse_css("hsl(1/3turn 100% 50%)"),
        Err(ColorError::InvalidSyntax)
    );
    assert_eq!(parse_css("hsl(30,40,50)"), Err(ColorError::InvalidSyntax));
}

#[test]
fn specified_css_modern_spaces_are_parsed_with_percent_reference_ranges() {
    let lab = parse_css("lab(50% 40% -25% / 25%)").unwrap();
    assert_eq!(lab.space(), ColorSpace::Lab);
    near(lab.channels()[0], 50.0, 0.0);
    near(lab.channels()[1], 50.0, 0.0);
    near(lab.channels()[2], -31.25, 0.0);
    near(lab.alpha(), 0.25, 0.0);

    let lch = parse_css("lch(70% 50% 0.5turn)").unwrap();
    assert_eq!(lch.space(), ColorSpace::Lch);
    near(lch.channels()[1], 75.0, 0.0);
    near(lch.channels()[2], 180.0, 0.0);

    let ok = parse_css("oklch(64% 0.16 180deg / 75%)").unwrap();
    assert_eq!(ok.space(), ColorSpace::Oklch);
    near(ok.channels()[0], 0.64, 0.0);
    near(ok.channels()[1], 0.16, 0.0);
    near(ok.channels()[2], 180.0, 0.0);
    near(ok.alpha(), 0.75, 0.0);

    let ok_lab = parse_css("oklab(50% 10% -10%)").unwrap();
    near(ok_lab.channels()[0], 0.5, 0.0);
    near(ok_lab.channels()[1], 0.04, 0.0);
    near(ok_lab.channels()[2], -0.04, 0.0);

    let p3 = parse_css("color(display-p3 0.2 0.4 0.7 / 0.3)").unwrap();
    assert_eq!(p3.space(), ColorSpace::DisplayP3);
    assert_eq!(p3.channels(), [0.2, 0.4, 0.7]);
    near(p3.alpha(), 0.3, 0.0);
    let xyz = parse_css("color(xyz 0.3 0.4 0.2)").unwrap();
    assert_eq!(xyz.space(), ColorSpace::XyzD65);
}

#[test]
fn common_names_transparent_and_css_round_trip() {
    assert_eq!(
        format_hex(parse_css("rebeccapurple").unwrap(), GamutMap::Clip).unwrap(),
        "#663399"
    );
    assert_eq!(
        format_hex(parse_css("TRANSPARENT").unwrap(), GamutMap::Clip).unwrap(),
        "#00000000"
    );
    for source in [
        "#2e4a9bca",
        "rgb(10.5 20.7 30.2 / 0.8)",
        "hsl(20 55% 42% / 0.25)",
        "hwb(220 10% 15%)",
        "lab(50 30 -20)",
        "lch(70 40 250)",
        "oklab(0.6 0.04 -0.1)",
        "oklch(0.6 0.1 270)",
        "color(display-p3 0.3 0.4 0.8 / 0.3)",
        "color(srgb-linear 0.2 0.4 0.7)",
    ] {
        let a = parse_css(source).unwrap();
        let serialized = format_css(a).unwrap();
        let b = parse_css(&serialized).unwrap();
        assert_eq!(a.space(), b.space(), "space from {source}");
        for i in 0..3 {
            near(a.channels()[i], b.channels()[i], 2e-14);
        }
        near(a.alpha(), b.alpha(), 1e-14);
    }
}

#[test]
fn malformed_or_unsupported_css_is_rejected() {
    for source in [
        "",
        "#f",
        "#abcdefe",
        "#ggff00",
        "rgb()",
        "rgb(1 2)",
        "rgb(1 2 3 4)",
        "rgb(1, 2 3)",
        "rgb(1 2 3 / 0.5 / 0.5)",
        "rgb(1 2 3) garbage",
        "hsl(20 50 50)",
        "color(display-p3 1 2)",
        "rgb(none 0 0)",
        "oklch(0.5 none 10)",
        "color(srgb 1 var(--g) 0)",
        "color(from red srgb r g b)",
        "rgb(calc(20 + 5) 0 0)",
        "color(prophoto-rgb 1 0 0)",
        "hwb(10, 0%, 0%)",
        "rgb(NaN 0 0)",
        "rgb(inf 0 0)",
    ] {
        assert!(parse_css(source).is_err(), "unexpected parse: {source}");
    }
    assert!(parse_css(&"a".repeat(2000)).is_err());
    assert!(parse_css("rgb(1 2 3)\0").is_err());
}

#[test]
fn hsl_hwb_hsv_numeric_conversion_matches_control_reference() {
    let green = Color::new(ColorSpace::Hsl, [120.0, 100.0, 50.0], 0.2).unwrap();
    rgb(green, [0.0, 1.0, 0.0]);
    near(green.to(ColorSpace::Srgb).unwrap().alpha(), 0.2, 0.0);
    let blue = Color::new(ColorSpace::Hsv, [240.0, 100.0, 100.0], 1.0).unwrap();
    rgb(blue, [0.0, 0.0, 1.0]);
    let restored = Color::new(ColorSpace::Srgb, [0.3, 0.6, 0.9], 1.0).unwrap();
    for space in [ColorSpace::Hsl, ColorSpace::Hwb, ColorSpace::Hsv] {
        let converted = restored.to(space).unwrap();
        let returned = converted.to(ColorSpace::Srgb).unwrap();
        rgb(returned, restored.channels());
    }
}
