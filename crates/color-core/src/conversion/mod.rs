//! Color-space conversion pipeline with original, dependency-free Rust code.
//!
//! Matrices, transfer functions and reference whites follow W3C CSS Color 4,
//! Candidate Recommendation Draft, 1 September 2026.
//! https://www.w3.org/TR/2026/CRD-css-color-4-20260901/
//!
//! The pipeline routes through XYZ D65, adapting to/from D50 only as needed.
//! RGB channels are not clipped; all intermediate calculations are f64.

use crate::cylindrical::{
    hsl_to_srgb, hsv_to_srgb, hwb_to_srgb, srgb_to_hsl, srgb_to_hsv, srgb_to_hwb,
};
use crate::math::{decode_rec2020, decode_srgb, encode_rec2020, encode_srgb, Matrix3, D50, D65};
use crate::spaces::{Color, ColorError, ColorSpace};

const SRGB_TO_XYZ: Matrix3 = Matrix3([
    [506752.0 / 1228815.0, 87881.0 / 245763.0, 12673.0 / 70218.0],
    [87098.0 / 409605.0, 175762.0 / 245763.0, 12673.0 / 175545.0],
    [7918.0 / 409605.0, 87881.0 / 737289.0, 1001167.0 / 1053270.0],
]);

const XYZ_TO_SRGB: Matrix3 = Matrix3([
    [12831.0 / 3959.0, -329.0 / 214.0, -1974.0 / 3959.0],
    [
        -851781.0 / 878810.0,
        1648619.0 / 878810.0,
        36519.0 / 878810.0,
    ],
    [705.0 / 12673.0, -2585.0 / 12673.0, 705.0 / 667.0],
]);

const P3_TO_XYZ: Matrix3 = Matrix3([
    [
        608311.0 / 1250200.0,
        189793.0 / 714400.0,
        198249.0 / 1000160.0,
    ],
    [
        35783.0 / 156275.0,
        247089.0 / 357200.0,
        198249.0 / 2500400.0,
    ],
    [0.0, 32229.0 / 714400.0, 5220557.0 / 5000800.0],
]);

const XYZ_TO_P3: Matrix3 = Matrix3([
    [
        446124.0 / 178915.0,
        -333277.0 / 357830.0,
        -72051.0 / 178915.0,
    ],
    [-14852.0 / 17905.0, 63121.0 / 35810.0, 423.0 / 17905.0],
    [11844.0 / 330415.0, -50337.0 / 660830.0, 316169.0 / 330415.0],
]);

const REC2020_TO_XYZ: Matrix3 = Matrix3([
    [
        63426534.0 / 99577255.0,
        20160776.0 / 139408157.0,
        47086771.0 / 278816314.0,
    ],
    [
        26158966.0 / 99577255.0,
        472592308.0 / 697040785.0,
        8267143.0 / 139408157.0,
    ],
    [0.0, 19567812.0 / 697040785.0, 295819943.0 / 278816314.0],
]);

const XYZ_TO_REC2020: Matrix3 = Matrix3([
    [
        30757411.0 / 17917100.0,
        -6372589.0 / 17917100.0,
        -4539589.0 / 17917100.0,
    ],
    [
        -19765991.0 / 29648200.0,
        47925759.0 / 29648200.0,
        467509.0 / 29648200.0,
    ],
    [
        792561.0 / 44930125.0,
        -1921689.0 / 44930125.0,
        42328811.0 / 44930125.0,
    ],
]);

const A98_TO_XYZ: Matrix3 = Matrix3([
    [
        573536.0 / 994567.0,
        263643.0 / 1420810.0,
        187206.0 / 994567.0,
    ],
    [
        591459.0 / 1989134.0,
        6239551.0 / 9945670.0,
        374412.0 / 4972835.0,
    ],
    [
        53769.0 / 1989134.0,
        351524.0 / 4972835.0,
        4929758.0 / 4972835.0,
    ],
]);
const XYZ_TO_A98: Matrix3 = Matrix3([
    [
        1829569.0 / 896150.0,
        -506331.0 / 896150.0,
        -308931.0 / 896150.0,
    ],
    [
        -851781.0 / 878810.0,
        1648619.0 / 878810.0,
        36519.0 / 878810.0,
    ],
    [
        16779.0 / 1248040.0,
        -147721.0 / 1248040.0,
        1266979.0 / 1248040.0,
    ],
]);
// The published W3C ProPhoto RGB matrices are relative to D50.
#[allow(clippy::excessive_precision)]
const PROPHOTO_TO_XYZ_D50: Matrix3 = Matrix3([
    [
        0.79776664490064230,
        0.13518129740053308,
        0.03134773412839220,
    ],
    [
        0.28807482881940130,
        0.71183523424187300,
        0.00008993693872564,
    ],
    [0.0, 0.0, 0.82510460251046020],
]);
#[allow(clippy::excessive_precision)]
const XYZ_D50_TO_PROPHOTO: Matrix3 = Matrix3([
    [
        1.34578688164715830,
        -0.25557208737979464,
        -0.05110186497554526,
    ],
    [
        -0.54463070512490190,
        1.50824774284514680,
        0.02052744743642139,
    ],
    [0.0, 0.0, 1.21196754563894520],
]);
fn decode_a98(value: f64) -> f64 {
    value.signum() * value.abs().powf(563.0 / 256.0)
}
fn encode_a98(value: f64) -> f64 {
    value.signum() * value.abs().powf(256.0 / 563.0)
}
fn decode_prophoto(value: f64) -> f64 {
    if value.abs() <= 16.0 / 512.0 {
        value / 16.0
    } else {
        value.signum() * value.abs().powf(1.8)
    }
}
fn encode_prophoto(value: f64) -> f64 {
    if value.abs() < 1.0 / 512.0 {
        16.0 * value
    } else {
        value.signum() * value.abs().powf(1.0 / 1.8)
    }
}

// Linear Bradford chromatic adaptation per CSS Color 4.
const D65_TO_D50: Matrix3 = Matrix3([
    [
        1.0479297925449969,
        0.022946870601609652,
        -0.05019226628920524,
    ],
    [
        0.02962780877005599,
        0.9904344267538799,
        -0.017073799063418826,
    ],
    [
        -0.009243040646204504,
        0.015055191490298152,
        0.7518742814281371,
    ],
]);

const D50_TO_D65: Matrix3 = Matrix3([
    [0.955473421488075, -0.02309845494876471, 0.06325924320057072],
    [
        -0.0283697093338637,
        1.0099953980813041,
        0.021041441191917323,
    ],
    [
        0.012314014864481998,
        -0.020507649298898964,
        1.330365926242124,
    ],
]);

// Keep the official W3C decimal coefficients verbatim for traceability.
#[allow(clippy::excessive_precision)]
const XYZ_TO_LMS: Matrix3 = Matrix3([
    [0.8190224379967030, 0.3619062600528904, -0.1288737815209879],
    [0.0329836539323885, 0.9292868615863434, 0.0361446663506424],
    [0.0481771893596242, 0.2642395317527308, 0.6335478284694309],
]);

// Keep the official W3C decimal coefficients verbatim for traceability.
#[allow(clippy::excessive_precision)]
const LMS_TO_OKLAB: Matrix3 = Matrix3([
    [0.2104542683093140, 0.7936177747023054, -0.0040720430116193],
    [1.9779985324311684, -2.4285922420485799, 0.4505937096174110],
    [0.0259040424655478, 0.7827717124575296, -0.8086757549230774],
]);

const OKLAB_TO_LMS: Matrix3 = Matrix3([
    [1.0, 0.3963377773761749, 0.2158037573099136],
    [1.0, -0.1055613458156586, -0.0638541728258133],
    [1.0, -0.0894841775298119, -1.2914855480194092],
]);

// Keep the official W3C decimal coefficients verbatim for traceability.
#[allow(clippy::excessive_precision)]
const LMS_TO_XYZ: Matrix3 = Matrix3([
    [1.2268798758459243, -0.5578149944602171, 0.2813910456659647],
    [-0.0405757452148008, 1.1122868032803170, -0.0717110580655164],
    [-0.0763729366746601, -0.4214933324022432, 1.5869240198367816],
]);

const EPSILON: f64 = 216.0 / 24389.0;
const KAPPA: f64 = 24389.0 / 27.0;

fn map3(values: [f64; 3], function: fn(f64) -> f64) -> [f64; 3] {
    [
        function(values[0]),
        function(values[1]),
        function(values[2]),
    ]
}

fn xyz_d50_to_lab(xyz: [f64; 3]) -> [f64; 3] {
    let scaled = [xyz[0] / D50[0], xyz[1] / D50[1], xyz[2] / D50[2]];
    let f = map3(scaled, |channel| {
        if channel > EPSILON {
            channel.cbrt()
        } else {
            (KAPPA * channel + 16.0) / 116.0
        }
    });
    [
        116.0 * f[1] - 16.0,
        500.0 * (f[0] - f[1]),
        200.0 * (f[1] - f[2]),
    ]
}

fn lab_to_xyz_d50(lab: [f64; 3]) -> [f64; 3] {
    let f = [
        (lab[0] + 16.0) / 116.0 + lab[1] / 500.0,
        (lab[0] + 16.0) / 116.0,
        (lab[0] + 16.0) / 116.0 - lab[2] / 200.0,
    ];
    let y = if lab[0] > KAPPA * EPSILON {
        f[1].powi(3)
    } else {
        lab[0] / KAPPA
    };
    let inverse = |value: f64| {
        if value.powi(3) > EPSILON {
            value.powi(3)
        } else {
            (116.0 * value - 16.0) / KAPPA
        }
    };
    [inverse(f[0]) * D50[0], y * D50[1], inverse(f[2]) * D50[2]]
}

fn xyz_d65_to_oklab(xyz: [f64; 3]) -> [f64; 3] {
    let lms = XYZ_TO_LMS.transform(xyz);
    LMS_TO_OKLAB.transform(map3(lms, f64::cbrt))
}

fn oklab_to_xyz_d65(oklab: [f64; 3]) -> [f64; 3] {
    let lms = OKLAB_TO_LMS.transform(oklab);
    LMS_TO_XYZ.transform(map3(lms, |value| value.powi(3)))
}

/// Convert Cartesian Lab/Oklab coordinates to polar LCH/OKLCH.
/// Exact zero chroma has no defined hue; this first numeric-only API uses
/// zero degrees by convention. CSS missing hues will be handled by a
/// separate future data representation, not silently parsed as zero.
fn to_polar(cartesian: [f64; 3]) -> [f64; 3] {
    let chroma = cartesian[1].hypot(cartesian[2]);
    let hue = if chroma < 1e-12 {
        0.0
    } else {
        cartesian[2]
            .atan2(cartesian[1])
            .to_degrees()
            .rem_euclid(360.0)
    };
    [cartesian[0], chroma, hue]
}

fn from_polar(polar: [f64; 3]) -> [f64; 3] {
    let angle = polar[2].to_radians();
    [polar[0], polar[1] * angle.cos(), polar[1] * angle.sin()]
}

fn to_xyz_d65(space: ColorSpace, c: [f64; 3]) -> [f64; 3] {
    match space {
        ColorSpace::Srgb => SRGB_TO_XYZ.transform(map3(c, decode_srgb)),
        ColorSpace::SrgbLinear => SRGB_TO_XYZ.transform(c),
        ColorSpace::DisplayP3 => P3_TO_XYZ.transform(map3(c, decode_srgb)),
        ColorSpace::DisplayP3Linear => P3_TO_XYZ.transform(c),
        ColorSpace::Rec2020 => REC2020_TO_XYZ.transform(map3(c, decode_rec2020)),
        ColorSpace::Rec2020Linear => REC2020_TO_XYZ.transform(c),
        ColorSpace::XyzD65 => c,
        ColorSpace::XyzD50 => D50_TO_D65.transform(c),
        ColorSpace::Lab => D50_TO_D65.transform(lab_to_xyz_d50(c)),
        ColorSpace::Lch => D50_TO_D65.transform(lab_to_xyz_d50(from_polar(c))),
        ColorSpace::LabD65 => {
            let xyz = lab_to_xyz_d50(c);
            [xyz[0] * D65[0] / D50[0], xyz[1], xyz[2] * D65[2] / D50[2]]
        }
        ColorSpace::Oklab => oklab_to_xyz_d65(c),
        ColorSpace::Oklch => oklab_to_xyz_d65(from_polar(c)),
        ColorSpace::Hsl => SRGB_TO_XYZ.transform(map3(hsl_to_srgb(c), decode_srgb)),
        ColorSpace::Hwb => SRGB_TO_XYZ.transform(map3(hwb_to_srgb(c), decode_srgb)),
        ColorSpace::Hsv => SRGB_TO_XYZ.transform(map3(hsv_to_srgb(c), decode_srgb)),
        ColorSpace::A98Rgb => A98_TO_XYZ.transform(map3(c, decode_a98)),
        ColorSpace::ProPhotoRgb => {
            D50_TO_D65.transform(PROPHOTO_TO_XYZ_D50.transform(map3(c, decode_prophoto)))
        }
    }
}

fn from_xyz_d65(target: ColorSpace, xyz: [f64; 3]) -> [f64; 3] {
    match target {
        ColorSpace::Srgb => map3(XYZ_TO_SRGB.transform(xyz), encode_srgb),
        ColorSpace::SrgbLinear => XYZ_TO_SRGB.transform(xyz),
        ColorSpace::DisplayP3 => map3(XYZ_TO_P3.transform(xyz), encode_srgb),
        ColorSpace::DisplayP3Linear => XYZ_TO_P3.transform(xyz),
        ColorSpace::Rec2020 => map3(XYZ_TO_REC2020.transform(xyz), encode_rec2020),
        ColorSpace::Rec2020Linear => XYZ_TO_REC2020.transform(xyz),
        ColorSpace::XyzD65 => xyz,
        ColorSpace::XyzD50 => D65_TO_D50.transform(xyz),
        ColorSpace::Lab => xyz_d50_to_lab(D65_TO_D50.transform(xyz)),
        ColorSpace::Lch => to_polar(xyz_d50_to_lab(D65_TO_D50.transform(xyz))),
        ColorSpace::LabD65 => xyz_d50_to_lab([
            xyz[0] * D50[0] / D65[0],
            xyz[1],
            xyz[2] * D50[2] / D65[2],
        ]),
        ColorSpace::Oklab => xyz_d65_to_oklab(xyz),
        ColorSpace::Oklch => to_polar(xyz_d65_to_oklab(xyz)),
        ColorSpace::Hsl => srgb_to_hsl(map3(XYZ_TO_SRGB.transform(xyz), encode_srgb)),
        ColorSpace::Hwb => srgb_to_hwb(map3(XYZ_TO_SRGB.transform(xyz), encode_srgb)),
        ColorSpace::Hsv => srgb_to_hsv(map3(XYZ_TO_SRGB.transform(xyz), encode_srgb)),
        ColorSpace::A98Rgb => map3(XYZ_TO_A98.transform(xyz), encode_a98),
        ColorSpace::ProPhotoRgb => map3(
            XYZ_D50_TO_PROPHOTO.transform(D65_TO_D50.transform(xyz)),
            encode_prophoto,
        ),
    }
}

/// Convert a color between coordinate spaces without clipping, gamut mapping,
/// CSS parsing, loss of f64 channel precision, or modification of alpha.
pub fn convert(input: Color, target: ColorSpace) -> Result<Color, ColorError> {
    if input.space() == target {
        return Ok(input);
    }
    let xyz = to_xyz_d65(input.space(), input.channels());
    if xyz.iter().any(|value| !value.is_finite()) {
        return Err(ColorError::NonFiniteResult);
    }
    let channels = from_xyz_d65(target, xyz);
    if channels.iter().any(|value| !value.is_finite()) {
        return Err(ColorError::NonFiniteResult);
    }
    Color::new(target, channels, input.alpha())
}
