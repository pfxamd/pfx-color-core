//! Small, explicitly tested numeric primitives used by color conversions.

/// A row-major 3x3 matrix for homogeneous-free color transformations.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Matrix3(pub [[f64; 3]; 3]);

impl Matrix3 {
    /// Multiply a 3-component column vector, without rounding or clipping.
    #[must_use]
    pub fn transform(self, input: [f64; 3]) -> [f64; 3] {
        let m = self.0;
        [
            m[0][0] * input[0] + m[0][1] * input[1] + m[0][2] * input[2],
            m[1][0] * input[0] + m[1][1] * input[1] + m[1][2] * input[2],
            m[2][0] * input[0] + m[2][1] * input[1] + m[2][2] * input[2],
        ]
    }
}

/// Canonical CIE D50 white from CSS Color 4 xy chromaticities.
pub const D50: [f64; 3] = [0.3457 / 0.3585, 1.0, (1.0 - 0.3457 - 0.3585) / 0.3585];

/// Canonical CIE D65 white from CSS Color 4 xy chromaticities.
pub const D65: [f64; 3] = [0.3127 / 0.3290, 1.0, (1.0 - 0.3127 - 0.3290) / 0.3290];

/// Signed sRGB transfer function: encoded RGB -> linear light.
#[must_use]
pub fn decode_srgb(channel: f64) -> f64 {
    if channel.abs() <= 0.04045 {
        channel / 12.92
    } else {
        channel.signum() * ((channel.abs() + 0.055) / 1.055).powf(2.4)
    }
}

/// Signed inverse sRGB transfer function: linear light -> encoded RGB.
#[must_use]
pub fn encode_srgb(channel: f64) -> f64 {
    if channel.abs() <= 0.0031308 {
        12.92 * channel
    } else {
        channel.signum() * (1.055 * channel.abs().powf(1.0 / 2.4) - 0.055)
    }
}

/// CSS Color 4 Rec.2020 encoding follows BT.1886 gamma 2.4.
/// This is not the BT.2020 camera OETF.
#[must_use]
pub fn decode_rec2020(channel: f64) -> f64 {
    channel.signum() * channel.abs().powf(2.4)
}

/// Inverse of the CSS Rec.2020 transfer function.
#[must_use]
pub fn encode_rec2020(channel: f64) -> f64 {
    channel.signum() * channel.abs().powf(1.0 / 2.4)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matrix_multiplies_column_vector() {
        let identity = Matrix3([[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]);
        assert_eq!(identity.transform([0.2, -0.3, 0.8]), [0.2, -0.3, 0.8]);
    }

    #[test]
    fn signed_transfer_functions_round_trip_extended_channels() {
        for x in [-1.25, -0.5, -0.0404, 0.0, 0.0404, 0.5, 1.25] {
            let value = encode_srgb(decode_srgb(x));
            assert!((value - x).abs() < 1e-12, "sRGB failed for {x}");
            let value = encode_rec2020(decode_rec2020(x));
            assert!((value - x).abs() < 1e-12, "Rec.2020 failed for {x}");
        }
    }

    #[test]
    fn srgb_specified_breakpoint_has_documented_numerical_discontinuity() {
        // CSS Color 4 specifies separately rounded switch thresholds
        // (0.04045 and 0.0031308) so the two branches are not exact inverses
        // at the breakpoint. This is not a reason to rewrite the standard.
        let boundary = encode_srgb(decode_srgb(0.04045));
        assert!((boundary - 0.04045).abs() < 5e-8);
    }
}
