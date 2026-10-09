//! CSS Color 4 missing-component semantics, kept separate from numeric Color.
//! A missing channel is never silently represented as a numeric zero in this type.
//! https://www.w3.org/TR/css-color-4/#interpolation-missing

use crate::css::{format_css, parse_css, MAX_CSS_INPUT};
use crate::interpolation::{interpolate, HueMethod};
use crate::spaces::{Color, ColorError, ColorSpace};

/// Bits 0..2 represent channels, bit 3 represents alpha.
pub const MISSING_ALL: u8 = 0b1111;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CssColor {
    value: Color,
    missing: u8,
}

impl CssColor {
    /// Construct from finite numeric channels plus a missing-component mask.
    /// Missing values are replaced with zero only in the stored *numeric* color.
    pub fn new(value: Color, missing: u8) -> Result<Self, ColorError> {
        if missing & !MISSING_ALL != 0 {
            return Err(ColorError::InvalidSyntax);
        }
        let mut channels = value.channels();
        for (index, channel) in channels.iter_mut().enumerate() {
            if missing & (1 << index) != 0 {
                *channel = 0.0;
            }
        }
        let alpha = if missing & 8 != 0 { 0.0 } else { value.alpha() };
        Ok(Self {
            value: Color::new(value.space(), channels, alpha)?,
            missing,
        })
    }

    pub fn numeric(self) -> Color {
        self.value
    }
    pub fn missing_mask(self) -> u8 {
        self.missing
    }
    pub fn space(self) -> ColorSpace {
        self.value.space()
    }

    /// Ordinary conversions consume missing input as zero per CSS Color 4 4.4.
    /// The result has no carried missing flags: carry-forward is interpolation-only.
    pub fn convert_numeric(self, target: ColorSpace) -> Result<Color, ColorError> {
        self.value.to(target)
    }
}

/// Only absolute CSS colors are parsed. Legacy comma syntax cannot contain none.
/// CSS calc(), relative colors and var() are deliberately not resolved here.
pub fn parse_css_missing(input: &str) -> Result<CssColor, ColorError> {
    let source = input.trim();
    if source.is_empty() || source.len() > MAX_CSS_INPUT || !source.is_ascii() {
        return Err(ColorError::InvalidSyntax);
    }
    let source = source.to_ascii_lowercase();
    if !source
        .split(|c: char| !c.is_ascii_alphabetic())
        .any(|w| w == "none")
    {
        return CssColor::new(parse_css(&source)?, 0);
    }
    let opening = source.find('(').ok_or(ColorError::InvalidSyntax)?;
    if !source.ends_with(')') {
        return Err(ColorError::InvalidSyntax);
    }
    let function = source[..opening].trim();
    let body = &source[opening + 1..source.len() - 1];
    if body.contains(',') || body.contains('(') || body.contains(')') {
        return Err(ColorError::UnsupportedSyntax);
    }
    let body = body.replace('/', " / ");
    let tokens: Vec<_> = body.split_whitespace().collect();
    let offset = usize::from(function == "color");
    if tokens.len() != offset + 3 && tokens.len() != offset + 5 {
        return Err(ColorError::InvalidSyntax);
    }
    if offset == 1 && tokens[0] == "none" {
        return Err(ColorError::InvalidSyntax);
    }
    let has_alpha = tokens.len() == offset + 5;
    if has_alpha && tokens[offset + 3] != "/" {
        return Err(ColorError::InvalidSyntax);
    }
    let mut missing = 0u8;
    let mut replaced = tokens.iter().map(|t| (*t).to_string()).collect::<Vec<_>>();
    for index in 0..3 {
        if tokens[offset + index] == "none" {
            missing |= 1 << index;
            replaced[offset + index] = if matches!(function, "hsl" | "hsla" | "hwb") && index > 0 {
                "0%".into()
            } else {
                "0".into()
            };
        }
    }
    if has_alpha && tokens[offset + 4] == "none" {
        missing |= 8;
        replaced[offset + 4] = "0".into();
    }
    if missing == 0 {
        return Err(ColorError::UnsupportedSyntax);
    }
    let sanitized = format!("{function}({})", replaced.join(" "));
    CssColor::new(parse_css(&sanitized)?, missing)
}

/// Serialize missing components as none, never numeric zero.
/// The numeric HSV/LabD65/non-CSS spaces cannot be serialized losslessly as CSS.
pub fn format_css_missing(input: CssColor) -> Result<String, ColorError> {
    let space = input.space();
    if !matches!(
        space,
        ColorSpace::Srgb
            | ColorSpace::Hsl
            | ColorSpace::Hwb
            | ColorSpace::Lab
            | ColorSpace::Lch
            | ColorSpace::Oklab
            | ColorSpace::Oklch
            | ColorSpace::SrgbLinear
            | ColorSpace::DisplayP3
            | ColorSpace::Rec2020
            | ColorSpace::A98Rgb
            | ColorSpace::ProPhotoRgb
            | ColorSpace::XyzD65
            | ColorSpace::XyzD50
    ) {
        return Err(ColorError::UnsupportedSyntax);
    }
    let rendered = format_css(input.numeric())?;
    if input.missing == 0 {
        return Ok(rendered);
    }
    let opening = rendered.find('(').ok_or(ColorError::InvalidSyntax)?;
    let function = &rendered[..opening];
    let body = rendered[opening + 1..rendered.len() - 1].replace('/', " / ");
    let mut parts: Vec<String> = body.split_whitespace().map(str::to_string).collect();
    let offset = usize::from(function == "color");
    for i in 0..3 {
        if input.missing & (1 << i) != 0 {
            parts[offset + i] = "none".into();
        }
    }
    if input.missing & 8 != 0 {
        if let Some(index) = parts.iter().position(|token| token == "/") {
            parts[index + 1] = "none".into();
        } else {
            parts.extend(["/".into(), "none".into()]);
        }
    }
    Ok(format!("{function}({})", parts.join(" ")))
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Component {
    Red,
    Green,
    Blue,
    Lightness,
    Chroma,
    Hue,
    A,
    B,
    Other,
}

fn components(space: ColorSpace) -> [Component; 3] {
    use Component::*;
    match space {
        ColorSpace::Srgb
        | ColorSpace::SrgbLinear
        | ColorSpace::DisplayP3
        | ColorSpace::DisplayP3Linear
        | ColorSpace::Rec2020
        | ColorSpace::Rec2020Linear
        | ColorSpace::A98Rgb
        | ColorSpace::ProPhotoRgb
        | ColorSpace::XyzD65
        | ColorSpace::XyzD50 => [Red, Green, Blue],
        ColorSpace::Lab | ColorSpace::LabD65 | ColorSpace::Oklab => [Lightness, A, B],
        ColorSpace::Lch | ColorSpace::Oklch => [Lightness, Chroma, Hue],
        ColorSpace::Hsl | ColorSpace::Hsv => [Hue, Chroma, Lightness],
        ColorSpace::Hwb => [Hue, Other, Other],
    }
}

/// Preserve individually analogous missing components and complete unmatched
/// analogous sets only during interpolation. Alpha always carries forward.
fn carry_mask(input: CssColor, destination: ColorSpace) -> u8 {
    let original = components(input.space());
    let target = components(destination);
    let mut result = input.missing & 8;
    let mut source_matched = [false; 3];
    let mut target_matched = [false; 3];
    for (i, source) in original.iter().enumerate() {
        for (j, destination) in target.iter().enumerate() {
            if *source != Component::Other && source == destination {
                source_matched[i] = true;
                target_matched[j] = true;
                if input.missing & (1 << i) != 0 {
                    result |= 1 << j;
                }
            }
        }
    }
    let source_remainder: Vec<_> = (0..3).filter(|i| !source_matched[*i]).collect();
    let dest_remainder: Vec<_> = (0..3).filter(|i| !target_matched[*i]).collect();
    if !source_remainder.is_empty()
        && !dest_remainder.is_empty()
        && source_remainder
            .iter()
            .all(|i| input.missing & (1 << i) != 0)
    {
        for index in dest_remainder {
            result |= 1 << index;
        }
    }
    result
}

/// Interpolate two CSS colors; missing channels borrow the other endpoint's
/// component. When BOTH endpoints are missing the result remains missing.
/// Underlying math is the existing tested numeric Rust interpolator.
pub fn interpolate_css_missing(
    first: CssColor,
    second: CssColor,
    fraction: f64,
    target: ColorSpace,
    hue_method: HueMethod,
) -> Result<CssColor, ColorError> {
    if !fraction.is_finite() || !(0.0..=1.0).contains(&fraction) {
        return Err(ColorError::InvalidFraction);
    }
    let m1 = carry_mask(first, target);
    let m2 = carry_mask(second, target);
    let mut a = first.numeric().to(target)?;
    let mut b = second.numeric().to(target)?;
    let mut ca = a.channels();
    let mut cb = b.channels();
    for index in 0..3 {
        let missing_a = m1 & (1 << index) != 0;
        let missing_b = m2 & (1 << index) != 0;
        if missing_a && !missing_b {
            ca[index] = cb[index];
        }
        if missing_b && !missing_a {
            cb[index] = ca[index];
        }
    }
    let mut aa = a.alpha();
    let mut ab = b.alpha();
    if m1 & 8 != 0 && m2 & 8 == 0 {
        aa = ab;
    }
    if m2 & 8 != 0 && m1 & 8 == 0 {
        ab = aa;
    }
    a = Color::new(target, ca, aa)?;
    b = Color::new(target, cb, ab)?;
    let result = interpolate(a, b, fraction, target, hue_method)?;
    CssColor::new(result, m1 & m2)
}
