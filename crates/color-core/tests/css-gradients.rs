use pfx_color_core::{
    Color, ColorError, ColorSpace, CssGradient, CssGradientKind, CssGradientOptions,
    CssRadialExtent, CssRadialShape, CssRadialSize, GamutMap, GradientStop, HueMethod,
};

fn close(value: f64, expected: f64) {
    assert!(
        (value - expected).abs() < 1e-9,
        "expected {expected}, got {value}"
    );
}
fn rgb(v: [f64; 3], alpha: f64) -> Color {
    Color::new(ColorSpace::Srgb, v, alpha).unwrap()
}
fn stops() -> [GradientStop; 2] {
    [
        GradientStop {
            position: 0.0,
            color: rgb([1.0, 0.0, 0.0], 1.0),
        },
        GradientStop {
            position: 1.0,
            color: rgb([0.0, 0.0, 1.0], 1.0),
        },
    ]
}
fn options(kind: CssGradientKind) -> CssGradientOptions {
    CssGradientOptions {
        width: 200.0,
        height: 100.0,
        kind,
        repeating: false,
        interpolation_space: ColorSpace::Srgb,
        target_space: ColorSpace::Srgb,
        hue_method: HueMethod::Shorter,
        gamut_map: GamutMap::Clip,
    }
}
fn linear(angle_degrees: f64) -> CssGradientKind {
    CssGradientKind::Linear { angle_degrees }
}
fn radial(shape: CssRadialShape, size: CssRadialSize) -> CssGradientKind {
    CssGradientKind::Radial {
        center_x: 100.0,
        center_y: 50.0,
        shape,
        size,
    }
}

#[test]
fn real_pixel_aspect_ratio_changes_css_linear_angle_projection() {
    let diagonal = CssGradient::new(&stops(), options(linear(45.0))).unwrap();
    close(diagonal.progress_at(0.0, 0.0).unwrap(), 1.0 / 3.0);
    close(diagonal.progress_at(200.0, 100.0).unwrap(), 2.0 / 3.0);
    close(diagonal.progress_at(0.0, 100.0).unwrap(), 0.0);
    close(diagonal.progress_at(200.0, 0.0).unwrap(), 1.0);
    let sample = diagonal.sample_pixel(0.0, 0.0).unwrap();
    close(sample.color.channels()[0], 2.0 / 3.0);
    close(sample.color.channels()[2], 1.0 / 3.0);
    let horizontal = CssGradient::new(&stops(), options(linear(90.0))).unwrap();
    close(horizontal.progress_at(100.0, 20.0).unwrap(), 0.5);
    close(horizontal.progress_at(200.0, 0.0).unwrap(), 1.0);
    let vertical = CssGradient::new(&stops(), options(linear(0.0))).unwrap();
    close(vertical.progress_at(100.0, 0.0).unwrap(), 1.0);
    close(vertical.progress_at(100.0, 100.0).unwrap(), 0.0);
}

#[test]
fn css_radial_shape_and_extent_change_real_box_radii() {
    let ellipse = CssGradient::new(
        &stops(),
        options(radial(
            CssRadialShape::Ellipse,
            CssRadialSize::Extent(CssRadialExtent::FarthestCorner),
        )),
    )
    .unwrap();
    close(ellipse.progress_at(100.0, 50.0).unwrap(), 0.0);
    close(ellipse.progress_at(200.0, 100.0).unwrap(), 1.0);
    close(
        ellipse.progress_at(200.0, 50.0).unwrap(),
        1.0 / 2.0_f64.sqrt(),
    );
    let circle = CssGradient::new(
        &stops(),
        options(radial(
            CssRadialShape::Circle,
            CssRadialSize::Extent(CssRadialExtent::FarthestCorner),
        )),
    )
    .unwrap();
    close(circle.progress_at(200.0, 100.0).unwrap(), 1.0);
    close(
        circle.progress_at(200.0, 50.0).unwrap(),
        100.0 / 12500.0_f64.sqrt(),
    );
    let near = CssGradient::new(
        &stops(),
        options(radial(
            CssRadialShape::Circle,
            CssRadialSize::Extent(CssRadialExtent::ClosestSide),
        )),
    )
    .unwrap();
    close(near.progress_at(200.0, 50.0).unwrap(), 2.0);
    let explicit = CssGradient::new(
        &stops(),
        options(radial(
            CssRadialShape::Ellipse,
            CssRadialSize::Explicit {
                radius_x: 40.0,
                radius_y: 20.0,
            },
        )),
    )
    .unwrap();
    close(explicit.progress_at(140.0, 50.0).unwrap(), 1.0);
    close(explicit.progress_at(100.0, 70.0).unwrap(), 1.0);
    close(explicit.progress_at(140.0, 70.0).unwrap(), 2.0_f64.sqrt());
}

#[test]
fn css_conic_rotates_clockwise_from_top_with_absolute_pixel_center() {
    let conic = CssGradient::new(
        &stops(),
        options(CssGradientKind::Conic {
            from_degrees: 30.0,
            center_x: 140.0,
            center_y: 40.0,
        }),
    )
    .unwrap();
    close(conic.progress_at(140.0, 20.0).unwrap(), 330.0 / 360.0);
    close(conic.progress_at(160.0, 40.0).unwrap(), 60.0 / 360.0);
    close(conic.progress_at(140.0, 40.0).unwrap(), 0.0);
    close(conic.progress_at(140.0, 60.0).unwrap(), 150.0 / 360.0);
}

#[test]
fn css_stop_fixup_preserves_order_and_hard_edges() {
    let colors = [
        GradientStop {
            position: -0.4,
            color: rgb([1.0, 0.0, 0.0], 1.0),
        },
        GradientStop {
            position: 0.6,
            color: rgb([0.0, 1.0, 0.0], 1.0),
        },
        GradientStop {
            position: 0.2,
            color: rgb([0.0, 0.0, 1.0], 1.0),
        },
        GradientStop {
            position: 1.4,
            color: rgb([0.0, 0.0, 0.0], 1.0),
        },
    ];
    let gradient = CssGradient::new(&colors, options(linear(90.0))).unwrap();
    assert_eq!(
        gradient
            .stops()
            .iter()
            .map(|s| s.position)
            .collect::<Vec<_>>(),
        vec![-0.4, 0.6, 0.6, 1.4]
    );
    assert_eq!(
        gradient.sample_progress(0.6).unwrap().color.channels(),
        [0.0, 0.0, 1.0]
    );
    assert_eq!(
        gradient.sample_progress(-100.0).unwrap().color.channels(),
        [1.0, 0.0, 0.0]
    );
    assert_eq!(
        gradient.sample_progress(100.0).unwrap().color.channels(),
        [0.0, 0.0, 0.0]
    );
}

#[test]
fn repeating_uses_first_to_last_interval_even_when_not_zero_to_one() {
    let stops = [
        GradientStop {
            position: 0.2,
            color: rgb([0.9, 0.0, 0.1], 1.0),
        },
        GradientStop {
            position: 0.4,
            color: rgb([0.0, 0.1, 0.9], 1.0),
        },
    ];
    let gradient = CssGradient::new(
        &stops,
        CssGradientOptions {
            repeating: true,
            ..options(linear(90.0))
        },
    )
    .unwrap();
    let first = gradient.sample_progress(0.3).unwrap().color;
    let next = gradient.sample_progress(0.5).unwrap().color;
    let before = gradient.sample_progress(-0.1).unwrap().color;
    assert_eq!(first, next);
    assert_eq!(first, before);
    close(first.channels()[0], 0.45);
    close(first.channels()[2], 0.5);
}

#[test]
fn repeating_zero_length_does_not_divide_by_zero() {
    let equal = [
        GradientStop {
            position: 0.4,
            color: rgb([1.0, 0.0, 0.0], 1.0),
        },
        GradientStop {
            position: 0.4,
            color: rgb([0.0, 0.0, 1.0], 1.0),
        },
    ];
    let grad = CssGradient::new(
        &equal,
        CssGradientOptions {
            repeating: true,
            ..options(linear(90.0))
        },
    )
    .unwrap();
    assert_eq!(
        grad.sample_progress(-0.5).unwrap().color.channels(),
        [0.5, 0.0, 0.5]
    );
    assert_eq!(
        grad.sample_progress(1.5).unwrap().color.channels(),
        [0.5, 0.0, 0.5]
    );
}


#[test]
fn repeating_zero_span_averages_three_stops_in_premultiplied_srgb() {
    // W3C CSS Images 3 reference: red 0px, white 0px, blue 0px
    // produces the uniform light purple rgb(75%, 50%, 75%).
    let three = [
        GradientStop { position: 0.4, color: rgb([1.0, 0.0, 0.0], 1.0) },
        GradientStop { position: 0.4, color: rgb([1.0, 1.0, 1.0], 1.0) },
        GradientStop { position: 0.4, color: rgb([0.0, 0.0, 1.0], 1.0) },
    ];
    let gradient = CssGradient::new(&three, CssGradientOptions {
        repeating: true, ..options(linear(90.0))
    }).unwrap();
    for p in [-12.0, 0.4, 20.0] {
        let color = gradient.sample_progress(p).unwrap().color;
        close(color.channels()[0], 0.75);
        close(color.channels()[1], 0.5);
        close(color.channels()[2], 0.75);
    }
    let transparent = [
        GradientStop { position: 0.4, color: rgb([1.0, 0.0, 0.0], 0.0) },
        GradientStop { position: 0.4, color: rgb([0.0, 0.0, 1.0], 1.0) },
    ];
    let gradient = CssGradient::new(&transparent, CssGradientOptions {
        repeating: true, ..options(linear(90.0))
    }).unwrap();
    let avg = gradient.sample_progress(0.7).unwrap().color;
    close(avg.alpha(), 0.5);
    close(avg.channels()[0], 0.0);
    close(avg.channels()[2], 1.0);
}

#[test]
fn css_gradients_use_existing_premultiplied_alpha_math() {
    let colors = [
        GradientStop {
            position: 0.0,
            color: rgb([1.0, 0.0, 0.0], 0.0),
        },
        GradientStop {
            position: 1.0,
            color: rgb([0.0, 0.0, 1.0], 1.0),
        },
    ];
    let gradient = CssGradient::new(&colors, options(linear(90.0))).unwrap();
    let center = gradient.sample_pixel(100.0, 50.0).unwrap().color;
    close(center.alpha(), 0.5);
    close(center.channels()[0], 0.0);
    close(center.channels()[2], 1.0);
}

#[test]
fn geometry_validation_refuses_invalid_dimensions_and_radii() {
    for width in [0.0, f64::INFINITY, f64::NAN, -1.0] {
        let configuration = CssGradientOptions {
            width,
            ..options(linear(90.0))
        };
        assert_eq!(
            CssGradient::new(&stops(), configuration),
            Err(ColorError::InvalidRange)
        );
    }
    let circle = radial(
        CssRadialShape::Circle,
        CssRadialSize::Explicit {
            radius_x: 40.0,
            radius_y: 10.0,
        },
    );
    assert_eq!(
        CssGradient::new(&stops(), options(circle)),
        Err(ColorError::InvalidRange)
    );
    let radial = radial(
        CssRadialShape::Ellipse,
        CssRadialSize::Explicit {
            radius_x: 0.0,
            radius_y: 20.0,
        },
    );
    assert_eq!(
        CssGradient::new(&stops(), options(radial)),
        Err(ColorError::InvalidRange)
    );
    let grad = CssGradient::new(&stops(), options(linear(40.0))).unwrap();
    assert_eq!(
        grad.sample_pixel(f64::NAN, 40.0),
        Err(ColorError::InvalidPosition)
    );
    assert_eq!(
        grad.sample_progress(f64::INFINITY),
        Err(ColorError::InvalidPosition)
    );
}
