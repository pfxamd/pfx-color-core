use pfx_color_core::{
    anchored_palette, generate_custom_harmony, generate_harmony, ramp_palette, tonal_palette,
    Color, ColorError, ColorSpace, GamutMap, Gradient, GradientKind, GradientOptions, GradientStop,
    HarmonyOptions, HarmonyScheme, RampOptions, TonalOptions,
};

fn c(space: ColorSpace, channels: [f64; 3]) -> Color {
    Color::new(space, channels, 1.0).unwrap()
}

fn close(actual: f64, expected: f64, tolerance: f64) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "expected {expected:.12}, got {actual:.12} (tolerance {tolerance})"
    );
}

fn channel_close(actual: Color, expected: [f64; 3], tolerance: f64) {
    for (a, e) in actual.channels().iter().zip(expected) {
        close(*a, e, tolerance);
    }
}

fn gradient_options() -> GradientOptions {
    GradientOptions {
        interpolation_space: ColorSpace::Srgb,
        ..GradientOptions::default()
    }
}

#[test]
fn tonal_palette_default_produces_nine_ordered_reference_space_steps() {
    let seed = Color::new(ColorSpace::Oklch, [0.5, 0.0, 220.0], 0.7).unwrap();
    let result = tonal_palette(seed, TonalOptions::default()).unwrap();
    assert_eq!(result.colors.len(), 9);
    assert_eq!(result.mode, pfx_color_core::PaletteMode::Tonal);
    close(result.colors[0].position, 0.0, 0.0);
    close(result.colors[8].position, 1.0, 0.0);
    for (index, step) in result.colors.iter().enumerate() {
        assert_eq!(step.index, index);
        assert!(!step.mapped);
        close(step.color.alpha(), 0.7, 0.0);
        close(
            step.color.to(ColorSpace::Oklch).unwrap().channels()[0],
            0.12 + (0.96 - 0.12) * step.position,
            1e-6,
        );
    }
}

#[test]
fn tonal_palette_validates_bounds_count_and_chroma() {
    let seed = c(ColorSpace::Srgb, [0.4, 0.5, 0.6]);
    for count in [0, 1, 257, usize::MAX] {
        assert_eq!(
            tonal_palette(
                seed,
                TonalOptions {
                    count,
                    ..TonalOptions::default()
                }
            ),
            Err(ColorError::InvalidCount)
        );
    }
    for (min, max) in [(0.7, 0.7), (0.8, 0.2), (-0.1, 0.5), (0.4, 1.1)] {
        assert_eq!(
            tonal_palette(
                seed,
                TonalOptions {
                    min_lightness: min,
                    max_lightness: max,
                    ..TonalOptions::default()
                }
            ),
            Err(ColorError::InvalidRange)
        );
    }
    for scale in [-0.1, f64::NAN, f64::INFINITY] {
        assert_eq!(
            tonal_palette(
                seed,
                TonalOptions {
                    chroma_scale: scale,
                    ..TonalOptions::default()
                }
            ),
            Err(ColorError::InvalidRange)
        );
    }
}

#[test]
fn tonal_palette_perceptually_maps_out_of_gamut_colors_explicitly() {
    let seed = c(ColorSpace::Oklch, [0.55, 0.4, 25.0]);
    let result = tonal_palette(
        seed,
        TonalOptions {
            count: 7,
            ..TonalOptions::default()
        },
    )
    .unwrap();
    assert_eq!(result.colors.len(), 7);
    assert!(result.colors.iter().any(|step| step.mapped));
    for step in result.colors {
        assert_eq!(step.color.space(), ColorSpace::Srgb);
        assert!(step
            .color
            .channels()
            .iter()
            .all(|v| (0.0..=1.0).contains(v)));
    }
}

#[test]
fn ramp_palette_preserves_endpoints_positions_and_alpha() {
    let a = Color::new(ColorSpace::Srgb, [1.0, 0.0, 0.0], 0.3).unwrap();
    let b = Color::new(ColorSpace::Srgb, [0.0, 0.0, 1.0], 0.9).unwrap();
    let result = ramp_palette(
        a,
        b,
        RampOptions {
            count: 5,
            interpolation_space: ColorSpace::Srgb,
            ..RampOptions::default()
        },
    )
    .unwrap();
    assert_eq!(result.colors.len(), 5);
    assert_eq!(result.colors[0].color, a);
    assert_eq!(result.colors[4].color, b);
    close(result.colors[2].position, 0.5, 0.0);
    let midpoint = result.colors[2].color;
    close(midpoint.alpha(), 0.6, 1e-12);
    channel_close(midpoint, [0.25, 0.0, 0.75], 1e-12);
}

#[test]
fn anchored_palette_matches_endpoints_and_exact_middle_anchor() {
    let anchors = [
        c(ColorSpace::Srgb, [1.0, 0.0, 0.0]),
        c(ColorSpace::Srgb, [0.0, 1.0, 0.0]),
        c(ColorSpace::Srgb, [0.0, 0.0, 1.0]),
    ];
    let opts = RampOptions {
        count: 5,
        interpolation_space: ColorSpace::Srgb,
        ..RampOptions::default()
    };
    let palette = anchored_palette(&anchors, opts).unwrap();
    assert_eq!(palette.colors.len(), 5);
    assert_eq!(palette.colors[0].color, anchors[0]);
    assert_eq!(palette.colors[2].color, anchors[1]);
    assert_eq!(palette.colors[4].color, anchors[2]);
    channel_close(palette.colors[1].color, [0.5, 0.5, 0.0], 1e-12);
    channel_close(palette.colors[3].color, [0.0, 0.5, 0.5], 1e-12);
}

#[test]
fn anchor_palette_rejects_insufficient_anchors_and_invalid_capacity() {
    let red = c(ColorSpace::Srgb, [1.0, 0.0, 0.0]);
    assert_eq!(
        anchored_palette(&[red], RampOptions::default()),
        Err(ColorError::InvalidCount)
    );
    assert_eq!(
        anchored_palette(
            &[red, red, red],
            RampOptions {
                count: 2,
                ..RampOptions::default()
            }
        ),
        Err(ColorError::InvalidCount)
    );
    assert_eq!(
        ramp_palette(
            red,
            red,
            RampOptions {
                count: 257,
                ..RampOptions::default()
            }
        ),
        Err(ColorError::InvalidCount)
    );
}

#[test]
fn palette_generation_is_deterministic_and_has_no_global_randomness() {
    let first = c(ColorSpace::Srgb, [0.2, 0.4, 0.6]);
    let second = c(ColorSpace::Srgb, [0.9, 0.6, 0.2]);
    assert_eq!(
        ramp_palette(first, second, RampOptions::default()).unwrap(),
        ramp_palette(first, second, RampOptions::default()).unwrap()
    );
}

#[test]
fn six_named_harmony_schemes_generate_expected_offsets() {
    let seed = c(ColorSpace::Oklch, [0.65, 0.1, 20.0]);
    let options = HarmonyOptions {
        target_space: ColorSpace::Oklch,
        ..HarmonyOptions::default()
    };
    let pairs = [
        (HarmonyScheme::Analogous, vec![-30.0, 0.0, 30.0]),
        (HarmonyScheme::Complementary, vec![0.0, 180.0]),
        (HarmonyScheme::SplitComplementary, vec![0.0, 150.0, 210.0]),
        (HarmonyScheme::Triadic, vec![0.0, 120.0, 240.0]),
        (HarmonyScheme::Tetradic, vec![0.0, 60.0, 180.0, 240.0]),
        (HarmonyScheme::Square, vec![0.0, 90.0, 180.0, 270.0]),
    ];
    for (scheme, expected_offsets) in pairs {
        let harmony = generate_harmony(seed, scheme, options).unwrap();
        assert_eq!(harmony.scheme, scheme);
        close(harmony.base_hue, 20.0, 0.0);
        assert_eq!(harmony.colors.len(), expected_offsets.len());
        for (step, offset) in harmony.colors.iter().zip(expected_offsets) {
            close(step.hue_offset, offset, 0.0);
            close(
                step.color.channels()[2],
                (20.0 + offset).rem_euclid(360.0),
                1e-12,
            );
            assert!(!step.mapped);
        }
    }
}

#[test]
fn custom_harmony_wraps_hues_preserves_alpha_and_order() {
    let seed = Color::new(ColorSpace::Oklch, [0.6, 0.08, 350.0], 0.42).unwrap();
    let options = HarmonyOptions {
        target_space: ColorSpace::Oklch,
        ..HarmonyOptions::default()
    };
    let harmony = generate_custom_harmony(seed, &[-30.0, 0.0, 30.0], options).unwrap();
    assert_eq!(harmony.scheme, HarmonyScheme::Custom);
    assert_eq!(harmony.colors.len(), 3);
    for (color, expected_hue) in harmony.colors.iter().zip([320.0, 350.0, 20.0]) {
        close(color.color.channels()[2], expected_hue, 1e-12);
        close(color.color.alpha(), 0.42, 0.0);
    }
}

#[test]
fn harmony_rejects_invalid_angles_and_scheme() {
    let seed = c(ColorSpace::Oklch, [0.6, 0.1, 75.0]);
    assert_eq!(
        generate_harmony(seed, HarmonyScheme::Custom, HarmonyOptions::default()),
        Err(ColorError::InvalidScheme)
    );
    assert_eq!(
        generate_harmony(
            seed,
            HarmonyScheme::Analogous,
            HarmonyOptions {
                analogous_angle: f64::NAN,
                ..HarmonyOptions::default()
            },
        ),
        Err(ColorError::InvalidAngle)
    );
    assert_eq!(
        generate_custom_harmony(seed, &[0.0], HarmonyOptions::default()),
        Err(ColorError::InvalidCount)
    );
    assert_eq!(
        generate_custom_harmony(seed, &[0.0, f64::INFINITY], HarmonyOptions::default()),
        Err(ColorError::InvalidAngle)
    );
}

#[test]
fn harmony_explicitly_maps_gamut_without_changing_source() {
    let seed = Color::new(ColorSpace::Oklch, [0.65, 0.4, 20.0], 0.7).unwrap();
    let harmony = generate_harmony(
        seed,
        HarmonyScheme::Triadic,
        HarmonyOptions {
            target_space: ColorSpace::Srgb,
            gamut_map: GamutMap::Clip,
            ..HarmonyOptions::default()
        },
    )
    .unwrap();
    assert!(harmony.colors.iter().any(|step| step.mapped));
    for color in harmony.colors {
        assert_eq!(color.color.space(), ColorSpace::Srgb);
        close(color.color.alpha(), 0.7, 0.0);
        assert!(color
            .color
            .channels()
            .iter()
            .all(|v| (0.0..=1.0).contains(v)));
    }
    assert_eq!(seed.channels(), [0.65, 0.4, 20.0]);
}

#[test]
fn gradient_stops_are_stably_sorted_and_duplicate_position_is_hard_stop() {
    let red = c(ColorSpace::Srgb, [1.0, 0.0, 0.0]);
    let green = c(ColorSpace::Srgb, [0.0, 1.0, 0.0]);
    let yellow = c(ColorSpace::Srgb, [1.0, 1.0, 0.0]);
    let blue = c(ColorSpace::Srgb, [0.0, 0.0, 1.0]);
    let stops = [
        GradientStop {
            position: 1.0,
            color: blue,
        },
        GradientStop {
            position: 0.0,
            color: red,
        },
        GradientStop {
            position: 0.5,
            color: green,
        },
        GradientStop {
            position: 0.5,
            color: yellow,
        },
    ];
    let gradient = Gradient::new(&stops, gradient_options()).unwrap();
    assert_eq!(gradient.stops()[0].color, red);
    assert_eq!(gradient.stops()[1].color, green);
    assert_eq!(gradient.stops()[2].color, yellow);
    assert_eq!(gradient.stops()[3].color, blue);
    assert_eq!(gradient.sample(0.5).unwrap().color, yellow);
    let before = gradient.sample(0.499999).unwrap().color;
    assert!(before.channels()[1] > 0.99);
    assert!(before.channels()[0] < 0.01);
    let after = gradient.sample(0.500001).unwrap().color;
    assert!(after.channels()[0] > 0.99);
}

#[test]
fn gradient_samples_are_premultiplied_alpha_and_have_exact_endpoints() {
    let red = Color::new(ColorSpace::Srgb, [1.0, 0.0, 0.0], 0.0).unwrap();
    let blue = c(ColorSpace::Srgb, [0.0, 0.0, 1.0]);
    let gradient = Gradient::new(
        &[
            GradientStop {
                position: 0.0,
                color: red,
            },
            GradientStop {
                position: 1.0,
                color: blue,
            },
        ],
        gradient_options(),
    )
    .unwrap();
    assert_eq!(gradient.sample(0.0).unwrap().color, red);
    assert_eq!(gradient.sample(1.0).unwrap().color, blue);
    let halfway = gradient.sample(0.5).unwrap();
    channel_close(halfway.color, [0.0, 0.0, 1.0], 1e-12);
    close(halfway.color.alpha(), 0.5, 0.0);
}

#[test]
fn gradient_extends_first_and_last_colors_outside_stops() {
    let red = c(ColorSpace::Srgb, [1.0, 0.0, 0.0]);
    let blue = c(ColorSpace::Srgb, [0.0, 0.0, 1.0]);
    let gradient = Gradient::new(
        &[
            GradientStop {
                position: 0.2,
                color: red,
            },
            GradientStop {
                position: 0.8,
                color: blue,
            },
        ],
        gradient_options(),
    )
    .unwrap();
    assert_eq!(gradient.sample(0.0).unwrap().color, red);
    assert_eq!(gradient.sample(1.0).unwrap().color, blue);
    channel_close(gradient.sample(0.5).unwrap().color, [0.5, 0.0, 0.5], 1e-12);
}

#[test]
fn linear_geometry_respects_css_like_angles_on_unit_square() {
    let red = c(ColorSpace::Srgb, [1.0, 0.0, 0.0]);
    let blue = c(ColorSpace::Srgb, [0.0, 0.0, 1.0]);
    let stops = [
        GradientStop {
            position: 0.0,
            color: red,
        },
        GradientStop {
            position: 1.0,
            color: blue,
        },
    ];
    let right = Gradient::new(&stops, gradient_options()).unwrap();
    close(right.sample_xy(0.0, 0.5).unwrap().position, 0.0, 1e-12);
    close(right.sample_xy(0.5, 0.7).unwrap().position, 0.5, 1e-12);
    close(right.sample_xy(1.0, 0.5).unwrap().position, 1.0, 1e-12);
    let up = Gradient::new(
        &stops,
        GradientOptions {
            kind: GradientKind::Linear { angle_degrees: 0.0 },
            ..gradient_options()
        },
    )
    .unwrap();
    close(up.sample_xy(0.5, 0.0).unwrap().position, 1.0, 1e-12);
    close(up.sample_xy(0.5, 1.0).unwrap().position, 0.0, 1e-12);
}

#[test]
fn radial_geometry_uses_farthest_unit_square_corner() {
    let red = c(ColorSpace::Srgb, [1.0, 0.0, 0.0]);
    let blue = c(ColorSpace::Srgb, [0.0, 0.0, 1.0]);
    let radial = Gradient::new(
        &[
            GradientStop {
                position: 0.0,
                color: red,
            },
            GradientStop {
                position: 1.0,
                color: blue,
            },
        ],
        GradientOptions {
            kind: GradientKind::Radial {
                center_x: 0.5,
                center_y: 0.5,
            },
            ..gradient_options()
        },
    )
    .unwrap();
    close(radial.sample_xy(0.5, 0.5).unwrap().position, 0.0, 0.0);
    close(radial.sample_xy(0.0, 0.0).unwrap().position, 1.0, 1e-12);
    close(
        radial.sample_xy(1.0, 0.5).unwrap().position,
        0.5_f64.sqrt(),
        1e-12,
    );
}

#[test]
fn conic_geometry_uses_clockwise_rotation_from_up() {
    let white = c(ColorSpace::Srgb, [1.0, 1.0, 1.0]);
    let black = c(ColorSpace::Srgb, [0.0, 0.0, 0.0]);
    let stops = [
        GradientStop {
            position: 0.0,
            color: black,
        },
        GradientStop {
            position: 1.0,
            color: white,
        },
    ];
    let conic = Gradient::new(
        &stops,
        GradientOptions {
            kind: GradientKind::Conic {
                from_degrees: 0.0,
                center_x: 0.5,
                center_y: 0.5,
            },
            ..gradient_options()
        },
    )
    .unwrap();
    close(conic.sample_xy(0.5, 0.0).unwrap().position, 0.0, 1e-12);
    close(conic.sample_xy(1.0, 0.5).unwrap().position, 0.25, 1e-12);
    close(conic.sample_xy(0.5, 1.0).unwrap().position, 0.5, 1e-12);
    close(conic.sample_xy(0.0, 0.5).unwrap().position, 0.75, 1e-12);
    close(conic.sample_xy(0.5, 0.5).unwrap().position, 0.0, 0.0);
    let rotated = Gradient::new(
        &stops,
        GradientOptions {
            kind: GradientKind::Conic {
                from_degrees: 90.0,
                center_x: 0.5,
                center_y: 0.5,
            },
            ..gradient_options()
        },
    )
    .unwrap();
    close(rotated.sample_xy(1.0, 0.5).unwrap().position, 0.0, 1e-12);
}

#[test]
fn gradient_rejects_out_of_range_positions_and_nonfinite_geometry() {
    let color = c(ColorSpace::Srgb, [0.4, 0.4, 0.4]);
    let stops = [
        GradientStop {
            position: 0.0,
            color,
        },
        GradientStop {
            position: 1.0,
            color,
        },
    ];
    assert_eq!(
        Gradient::new(&stops[..1], gradient_options()),
        Err(ColorError::InvalidCount)
    );
    for position in [-0.001, 1.001, f64::NAN, f64::INFINITY] {
        let invalid = [
            GradientStop {
                position: 0.0,
                color,
            },
            GradientStop { position, color },
        ];
        assert_eq!(
            Gradient::new(&invalid, gradient_options()),
            Err(ColorError::InvalidPosition)
        );
    }
    for angle in [f64::NAN, f64::INFINITY] {
        assert_eq!(
            Gradient::new(
                &stops,
                GradientOptions {
                    kind: GradientKind::Linear {
                        angle_degrees: angle
                    },
                    ..gradient_options()
                }
            ),
            Err(ColorError::InvalidAngle)
        );
    }
    assert_eq!(
        Gradient::new(
            &stops,
            GradientOptions {
                kind: GradientKind::Radial {
                    center_x: 1.1,
                    center_y: 0.5
                },
                ..gradient_options()
            }
        ),
        Err(ColorError::InvalidPosition)
    );
    let gradient = Gradient::new(&stops, gradient_options()).unwrap();
    assert_eq!(gradient.sample(f64::NAN), Err(ColorError::InvalidPosition));
    assert_eq!(
        gradient.sample_xy(-0.1, 0.4),
        Err(ColorError::InvalidPosition)
    );
}

#[test]
fn gradients_are_deterministic_and_do_not_map_sources_implicitly() {
    let p3 = c(ColorSpace::DisplayP3, [0.0, 1.0, 0.0]);
    let black = c(ColorSpace::Srgb, [0.0, 0.0, 0.0]);
    let stops = [
        GradientStop {
            position: 0.0,
            color: p3,
        },
        GradientStop {
            position: 1.0,
            color: black,
        },
    ];
    let gradient = Gradient::new(&stops, gradient_options()).unwrap();
    assert!(gradient.sample(0.0).unwrap().mapped);
    assert_eq!(gradient.stops()[0].color, p3);
    assert_eq!(gradient.sample(0.4).unwrap(), gradient.sample(0.4).unwrap());
}

#[test]
fn maximum_palette_and_gradient_size_is_bounded() {
    let seed = c(ColorSpace::Oklch, [0.5, 0.0, 10.0]);
    assert_eq!(
        tonal_palette(
            seed,
            TonalOptions {
                count: 256,
                ..TonalOptions::default()
            }
        )
        .unwrap()
        .colors
        .len(),
        256
    );
    let stops = vec![
        GradientStop {
            position: 0.5,
            color: seed
        };
        257
    ];
    assert_eq!(
        Gradient::new(&stops, gradient_options()),
        Err(ColorError::InvalidCount)
    );
}
