//! Dependency-free cylindrical sRGB spaces for UI color controls.
//!
//! HSL/HWB channels: hue in degrees, saturation/lightness/whiteness/blackness
//! in 0..100 percentage units. HSV (non-CSS) is hue degrees plus saturation
//! and value in 0..100 units. Numeric conversions preserve alpha separately.

fn normalized_hue(h: f64) -> f64 {
    h.rem_euclid(360.0)
}

/// Pure hue in RGB, with saturation and value both one.
fn hue_rgb(hue: f64) -> [f64; 3] {
    let h = normalized_hue(hue) / 60.0;
    let x = 1.0 - (h.rem_euclid(2.0) - 1.0).abs();
    if h < 1.0 {
        [1.0, x, 0.0]
    } else if h < 2.0 {
        [x, 1.0, 0.0]
    } else if h < 3.0 {
        [0.0, 1.0, x]
    } else if h < 4.0 {
        [0.0, x, 1.0]
    } else if h < 5.0 {
        [x, 0.0, 1.0]
    } else {
        [1.0, 0.0, x]
    }
}

/// Standard HSL (0..100 for saturation/lightness) to encoded sRGB.
pub fn hsl_to_srgb([hue, saturation, lightness]: [f64; 3]) -> [f64; 3] {
    let s = saturation / 100.0;
    let l = lightness / 100.0;
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let m = l - c / 2.0;
    hue_rgb(hue).map(|channel| channel * c + m)
}

/// Encoded sRGB to HSL. Neutral hue is defined as zero in numeric space.
pub fn srgb_to_hsl([r, g, b]: [f64; 3]) -> [f64; 3] {
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;
    let l = (max + min) / 2.0;
    let s = if delta == 0.0 {
        0.0
    } else {
        delta / (1.0 - (2.0 * l - 1.0).abs())
    };
    [rgb_hue(r, g, b, max, delta), s * 100.0, l * 100.0]
}

fn rgb_hue(r: f64, g: f64, b: f64, max: f64, delta: f64) -> f64 {
    if delta == 0.0 {
        0.0
    } else if max == r {
        (60.0 * ((g - b) / delta)).rem_euclid(360.0)
    } else if max == g {
        (60.0 * ((b - r) / delta + 2.0)).rem_euclid(360.0)
    } else {
        (60.0 * ((r - g) / delta + 4.0)).rem_euclid(360.0)
    }
}

/// CSS HWB: if whiteness+blackness >= 100%, produce achromatic color.
pub fn hwb_to_srgb([hue, whiteness, blackness]: [f64; 3]) -> [f64; 3] {
    let w = whiteness / 100.0;
    let b = blackness / 100.0;
    if w + b >= 1.0 {
        let neutral = if w + b == 0.0 { 0.0 } else { w / (w + b) };
        return [neutral; 3];
    }
    hue_rgb(hue).map(|channel| channel * (1.0 - w - b) + w)
}

/// Encoded sRGB to HWB; blackness and whiteness in percent.
pub fn srgb_to_hwb([r, g, b]: [f64; 3]) -> [f64; 3] {
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    [
        rgb_hue(r, g, b, max, max - min),
        min * 100.0,
        (1.0 - max) * 100.0,
    ]
}

/// Non-CSS HSV as defined for familiar design UI controls.
pub fn hsv_to_srgb([hue, saturation, value]: [f64; 3]) -> [f64; 3] {
    let v = value / 100.0;
    let chroma = v * saturation / 100.0;
    hue_rgb(hue).map(|channel| channel * chroma + v - chroma)
}

/// Encoded sRGB to HSV; neutral hue is zero in numeric space.
pub fn srgb_to_hsv([r, g, b]: [f64; 3]) -> [f64; 3] {
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;
    let saturation = if max == 0.0 { 0.0 } else { delta / max };
    [
        rgb_hue(r, g, b, max, delta),
        saturation * 100.0,
        max * 100.0,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn near(a: f64, b: f64) {
        assert!((a - b).abs() < 1e-11, "{a} != {b}");
    }

    #[test]
    fn primaries_and_gray_are_stable() {
        for (h, rgb) in [
            (0.0, [1.0, 0.0, 0.0]),
            (120.0, [0.0, 1.0, 0.0]),
            (240.0, [0.0, 0.0, 1.0]),
            (360.0, [1.0, 0.0, 0.0]),
        ] {
            let actual = hsl_to_srgb([h, 100.0, 50.0]);
            for i in 0..3 {
                near(actual[i], rgb[i]);
            }
        }
        assert_eq!(srgb_to_hsl([0.5; 3]), [0.0, 0.0, 50.0]);
        assert_eq!(hwb_to_srgb([240.0, 60.0, 60.0]), [0.5; 3]);
    }

    #[test]
    fn roundtrip_ui_spaces() {
        for rgb in [[0.2, 0.4, 0.6], [0.9, 0.3, 0.05], [0.3, 0.3, 0.3]] {
            for reencoded in [
                hsl_to_srgb(srgb_to_hsl(rgb)),
                hwb_to_srgb(srgb_to_hwb(rgb)),
                hsv_to_srgb(srgb_to_hsv(rgb)),
            ] {
                for i in 0..3 {
                    near(rgb[i], reencoded[i]);
                }
            }
        }
    }
}
