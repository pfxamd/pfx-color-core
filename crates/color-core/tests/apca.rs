use pfx_color_core::{apca_contrast, Color, ColorError, ColorSpace};

fn color(channels: [f64; 3]) -> Color {
    Color::new(ColorSpace::Srgb, channels, 1.0).unwrap()
}

#[test]
fn apca_wcag_is_polarity_sensitive_not_symmetric() {
    let black = color([0.0, 0.0, 0.0]);
    let white = color([1.0, 1.0, 1.0]);
    let on_white = apca_contrast(black, white).unwrap();
    let on_black = apca_contrast(white, black).unwrap();
    assert!((on_white - 106.04).abs() < 0.1, "black/white {on_white}");
    assert!((on_black + 107.88).abs() < 0.1, "white/black {on_black}");
    assert!(on_white > 0.0);
    assert!(on_black < 0.0);
}

#[test]
fn apca_soft_clip_and_low_contrast_cutoff_are_applied() {
    let black = color([0.0, 0.0, 0.0]);
    assert_eq!(apca_contrast(black, black), Ok(0.0));
    assert_eq!(
        apca_contrast(black, color([0.01, 0.01, 0.01])),
        Ok(0.0)
    );
}

#[test]
fn apca_requires_explicit_opaque_in_gamut_colors() {
    let black = color([0.0, 0.0, 0.0]);
    let transparent = Color::new(ColorSpace::Srgb, [0.3, 0.2, 0.1], 0.5).unwrap();
    assert_eq!(apca_contrast(transparent, black), Err(ColorError::RequiresOpaque));
    let out_of_gamut = color([1.2, 0.4, 0.5]);
    assert_eq!(apca_contrast(out_of_gamut, black), Err(ColorError::OutOfGamut));
}
