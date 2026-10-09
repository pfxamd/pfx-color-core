//! Perceptual color differences.
//!
//! CIEDE2000 follows Sharma, Wu & Dalal (2005), using CIE Lab coordinates,
//! reference conditions k_L = k_C = k_H = 1. CIE76 and Delta-E OK are Euclidean
//! distances in CIE Lab and OKLab respectively.
//! Reference pairs: https://hajim.rochester.edu/ece/sites/gsharma/ciede2000/

use crate::spaces::{Color, ColorError, ColorSpace};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DifferenceMethod {
    Cie76,
    Ciede2000,
    Ok,
}

/// Return a nonnegative difference. Alpha does not participate: blend colors
/// against a known background first when transparency matters.
pub fn difference(a: Color, b: Color, method: DifferenceMethod) -> Result<f64, ColorError> {
    let space = match method {
        DifferenceMethod::Cie76 | DifferenceMethod::Ciede2000 => ColorSpace::Lab,
        DifferenceMethod::Ok => ColorSpace::Oklab,
    };
    let x = a.to(space)?.channels();
    let y = b.to(space)?.channels();
    let delta = match method {
        DifferenceMethod::Cie76 | DifferenceMethod::Ok => {
            ((x[0] - y[0]).powi(2) + (x[1] - y[1]).powi(2) + (x[2] - y[2]).powi(2)).sqrt()
        }
        DifferenceMethod::Ciede2000 => ciede2000(x, y),
    };
    if delta.is_finite() {
        Ok(delta)
    } else {
        Err(ColorError::NonFiniteResult)
    }
}

fn hue_degrees(a: f64, b: f64) -> f64 {
    b.atan2(a).to_degrees().rem_euclid(360.0)
}

fn cos_degrees(degrees: f64) -> f64 {
    degrees.to_radians().cos()
}

fn sin_degrees(degrees: f64) -> f64 {
    degrees.to_radians().sin()
}

/// CIEDE2000 with unity weighting factors and the specified hue-wrapping
/// behavior for zero chroma, complementary hues and crossing 360 degrees.
fn ciede2000(a: [f64; 3], b: [f64; 3]) -> f64 {
    let (l1, a1, b1) = (a[0], a[1], a[2]);
    let (l2, a2, b2) = (b[0], b[1], b[2]);
    let c1 = a1.hypot(b1);
    let c2 = a2.hypot(b2);
    let c_bar = (c1 + c2) / 2.0;
    let c7 = c_bar.powi(7);
    let g = 0.5 * (1.0 - (c7 / (c7 + 25.0_f64.powi(7))).sqrt());

    let a1_prime = (1.0 + g) * a1;
    let a2_prime = (1.0 + g) * a2;
    let c1_prime = a1_prime.hypot(b1);
    let c2_prime = a2_prime.hypot(b2);
    let h1_prime = hue_degrees(a1_prime, b1);
    let h2_prime = hue_degrees(a2_prime, b2);

    let delta_l = l2 - l1;
    let delta_c = c2_prime - c1_prime;
    let hue_diff = h2_prime - h1_prime;
    let delta_h_angle = if c1_prime * c2_prime == 0.0 {
        0.0
    } else if hue_diff > 180.0 {
        hue_diff - 360.0
    } else if hue_diff < -180.0 {
        hue_diff + 360.0
    } else {
        hue_diff
    };
    let delta_h = 2.0 * (c1_prime * c2_prime).sqrt() * sin_degrees(delta_h_angle / 2.0);

    let l_bar_prime = (l1 + l2) / 2.0;
    let c_bar_prime = (c1_prime + c2_prime) / 2.0;
    let h_bar_prime = if c1_prime * c2_prime == 0.0 {
        h1_prime + h2_prime
    } else if hue_diff.abs() <= 180.0 {
        (h1_prime + h2_prime) / 2.0
    } else if h1_prime + h2_prime < 360.0 {
        (h1_prime + h2_prime + 360.0) / 2.0
    } else {
        (h1_prime + h2_prime - 360.0) / 2.0
    };

    let t = 1.0 - 0.17 * cos_degrees(h_bar_prime - 30.0)
        + 0.24 * cos_degrees(2.0 * h_bar_prime)
        + 0.32 * cos_degrees(3.0 * h_bar_prime + 6.0)
        - 0.20 * cos_degrees(4.0 * h_bar_prime - 63.0);

    let delta_theta = 30.0 * (-((h_bar_prime - 275.0) / 25.0).powi(2)).exp();
    let c_bar_prime_7 = c_bar_prime.powi(7);
    let r_c = 2.0 * (c_bar_prime_7 / (c_bar_prime_7 + 25.0_f64.powi(7))).sqrt();
    let r_t = -sin_degrees(2.0 * delta_theta) * r_c;

    let s_l =
        1.0 + 0.015 * (l_bar_prime - 50.0).powi(2) / (20.0 + (l_bar_prime - 50.0).powi(2)).sqrt();
    let s_c = 1.0 + 0.045 * c_bar_prime;
    let s_h = 1.0 + 0.015 * c_bar_prime * t;
    let l_term = delta_l / s_l;
    let c_term = delta_c / s_c;
    let h_term = delta_h / s_h;
    (l_term.powi(2) + c_term.powi(2) + h_term.powi(2) + r_t * c_term * h_term)
        .max(0.0)
        .sqrt()
}
