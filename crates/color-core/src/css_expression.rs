//! Bounded absolute calc() and relative-color evaluation for opt-in CssColor.
//! This parser does not resolve var(), currentColor, profiles, or full CSS math.
//! Channel identifiers in relative colors are unitless numbers per CSS Color 4.
//! References: https://www.w3.org/TR/css-color-4/#relative-colors

use crate::css::MAX_CSS_INPUT;
use crate::css_missing::{parse_css_missing, CssColor};
use crate::spaces::{Color, ColorError, ColorSpace};

const MAX_DEPTH: usize = 16;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Unit {
    Number,
    Percent,
    Angle,
}

#[derive(Clone, Copy)]
struct Quantity {
    value: f64,
    unit: Unit,
}

impl Quantity {
    fn new(value: f64, unit: Unit) -> Result<Self, ColorError> {
        if !value.is_finite() {
            return Err(ColorError::InvalidSyntax);
        }
        Ok(Self { value, unit })
    }
    fn add(self, other: Self, subtract: bool) -> Result<Self, ColorError> {
        if self.unit != other.unit {
            return Err(ColorError::InvalidSyntax);
        }
        Self::new(
            self.value + if subtract { -other.value } else { other.value },
            self.unit,
        )
    }
    fn multiply(self, other: Self) -> Result<Self, ColorError> {
        match (self.unit, other.unit) {
            (Unit::Number, _) => Self::new(self.value * other.value, other.unit),
            (_, Unit::Number) => Self::new(self.value * other.value, self.unit),
            _ => Err(ColorError::InvalidSyntax),
        }
    }
    fn divide(self, other: Self) -> Result<Self, ColorError> {
        if other.value == 0.0 {
            return Err(ColorError::InvalidSyntax);
        }
        if other.unit != Unit::Number {
            return Err(ColorError::InvalidSyntax);
        }
        Self::new(self.value / other.value, self.unit)
    }
}

struct MathParser<'a> {
    source: &'a str,
    pos: usize,
    depth: usize,
    variables: &'a [(&'static str, f64)],
}

impl<'a> MathParser<'a> {
    fn new(source: &'a str, variables: &'a [(&'static str, f64)]) -> Self {
        Self {
            source,
            pos: 0,
            depth: 0,
            variables,
        }
    }
    fn rest(&self) -> &'a str {
        &self.source[self.pos..]
    }
    fn skip_space(&mut self) {
        while let Some(byte) = self.source.as_bytes().get(self.pos) {
            if byte.is_ascii_whitespace() {
                self.pos += 1;
            } else {
                break;
            }
        }
    }
    fn consume(&mut self, symbol: u8) -> bool {
        self.skip_space();
        if self.source.as_bytes().get(self.pos) == Some(&symbol) {
            self.pos += 1;
            true
        } else {
            false
        }
    }
    fn expression(&mut self) -> Result<Quantity, ColorError> {
        let mut result = self.product()?;
        loop {
            if self.consume(b'+') {
                result = result.add(self.product()?, false)?;
            } else if self.consume(b'-') {
                result = result.add(self.product()?, true)?;
            } else {
                return Ok(result);
            }
        }
    }
    fn product(&mut self) -> Result<Quantity, ColorError> {
        let mut result = self.unary()?;
        loop {
            if self.consume(b'*') {
                result = result.multiply(self.unary()?)?;
            } else if self.consume(b'/') {
                result = result.divide(self.unary()?)?;
            } else {
                return Ok(result);
            }
        }
    }
    fn unary(&mut self) -> Result<Quantity, ColorError> {
        if self.consume(b'+') {
            self.depth += 1;
            if self.depth > MAX_DEPTH {
                return Err(ColorError::UnsupportedSyntax);
            }
            let result = self.unary();
            self.depth -= 1;
            return result;
        }
        if self.consume(b'-') {
            self.depth += 1;
            if self.depth > MAX_DEPTH {
                return Err(ColorError::UnsupportedSyntax);
            }
            let result = self.unary();
            self.depth -= 1;
            let result = result?;
            return Quantity::new(-result.value, result.unit);
        }
        self.atom()
    }
    fn atom(&mut self) -> Result<Quantity, ColorError> {
        self.skip_space();
        let nested = if self.rest().starts_with("calc(") {
            self.pos += 5;
            true
        } else {
            self.consume(b'(')
        };
        if nested {
            self.depth += 1;
            if self.depth > MAX_DEPTH {
                return Err(ColorError::UnsupportedSyntax);
            }
            let value = self.expression()?;
            if !self.consume(b')') {
                return Err(ColorError::InvalidSyntax);
            }
            self.depth -= 1;
            return Ok(value);
        }
        let bytes = self.source.as_bytes();
        let start = self.pos;
        if bytes.get(start).is_some_and(u8::is_ascii_alphabetic) {
            while bytes.get(self.pos).is_some_and(u8::is_ascii_alphabetic) {
                self.pos += 1;
            }
            let ident = &self.source[start..self.pos];
            if self.rest().starts_with('(') {
                return Err(ColorError::UnsupportedSyntax);
            }
            return self
                .variables
                .iter()
                .find(|(name, _)| *name == ident)
                .map(|(_, value)| Quantity::new(*value, Unit::Number))
                .unwrap_or(Err(ColorError::UnsupportedSyntax));
        }
        let mut digits = 0;
        while bytes.get(self.pos).is_some_and(u8::is_ascii_digit) {
            self.pos += 1;
            digits += 1;
        }
        if bytes.get(self.pos) == Some(&b'.') {
            self.pos += 1;
            while bytes.get(self.pos).is_some_and(u8::is_ascii_digit) {
                self.pos += 1;
                digits += 1;
            }
        }
        if digits == 0 {
            return Err(ColorError::InvalidSyntax);
        }
        if matches!(bytes.get(self.pos), Some(b'e' | b'E')) {
            let exponent_start = self.pos;
            self.pos += 1;
            if matches!(bytes.get(self.pos), Some(b'+' | b'-')) {
                self.pos += 1;
            }
            let first = self.pos;
            while bytes.get(self.pos).is_some_and(u8::is_ascii_digit) {
                self.pos += 1;
            }
            if first == self.pos {
                self.pos = exponent_start;
            }
        }
        let number = self.source[start..self.pos]
            .parse::<f64>()
            .map_err(|_| ColorError::InvalidSyntax)?;
        let unit_start = self.pos;
        while bytes.get(self.pos).is_some_and(u8::is_ascii_alphabetic) {
            self.pos += 1;
        }
        if bytes.get(self.pos) == Some(&b'%') {
            self.pos += 1;
        }
        let unit_str = &self.source[unit_start..self.pos];
        let (unit, value) = match unit_str {
            "" => (Unit::Number, number),
            "%" => (Unit::Percent, number),
            "deg" => (Unit::Angle, number),
            "turn" => (Unit::Angle, number * 360.0),
            "grad" => (Unit::Angle, number * 0.9),
            "rad" => (Unit::Angle, number * 180.0 / std::f64::consts::PI),
            _ => return Err(ColorError::UnsupportedSyntax),
        };
        Quantity::new(value, unit)
    }
    fn evaluate(mut self) -> Result<Quantity, ColorError> {
        let result = self.expression()?;
        self.skip_space();
        if !self.rest().is_empty() {
            return Err(ColorError::InvalidSyntax);
        }
        Ok(result)
    }
}

/// Splits top-level slash and whitespace, retaining balanced nested functions.
fn top_tokens(source: &str) -> Result<Vec<&str>, ColorError> {
    let mut tokens = Vec::new();
    let mut depth = 0usize;
    let mut start = None;
    for (index, chr) in source.char_indices() {
        if chr == '/' && depth == 0 {
            if let Some(begin) = start.take() {
                tokens.push(&source[begin..index]);
            }
            tokens.push(&source[index..index + 1]);
        } else if chr.is_ascii_whitespace() && depth == 0 {
            if let Some(begin) = start.take() {
                tokens.push(&source[begin..index]);
            }
        } else {
            if start.is_none() {
                start = Some(index);
            }
            if chr == '(' {
                depth += 1;
                if depth > MAX_DEPTH {
                    return Err(ColorError::UnsupportedSyntax);
                }
            } else if chr == ')' {
                if depth == 0 {
                    return Err(ColorError::InvalidSyntax);
                }
                depth -= 1;
            }
        }
    }
    if depth != 0 {
        return Err(ColorError::InvalidSyntax);
    }
    if let Some(begin) = start {
        tokens.push(&source[begin..]);
    }
    Ok(tokens)
}

fn color_space(kind: &str, model: Option<&str>) -> Result<ColorSpace, ColorError> {
    match (kind, model) {
        ("rgb" | "rgba", None) => Ok(ColorSpace::Srgb),
        ("hsl" | "hsla", None) => Ok(ColorSpace::Hsl),
        ("hwb", None) => Ok(ColorSpace::Hwb),
        ("lab", None) => Ok(ColorSpace::Lab),
        ("lch", None) => Ok(ColorSpace::Lch),
        ("oklab", None) => Ok(ColorSpace::Oklab),
        ("oklch", None) => Ok(ColorSpace::Oklch),
        ("color", Some("srgb")) => Ok(ColorSpace::Srgb),
        ("color", Some("srgb-linear")) => Ok(ColorSpace::SrgbLinear),
        ("color", Some("display-p3")) => Ok(ColorSpace::DisplayP3),
        ("color", Some("rec2020")) => Ok(ColorSpace::Rec2020),
        ("color", Some("a98-rgb")) => Ok(ColorSpace::A98Rgb),
        ("color", Some("prophoto-rgb")) => Ok(ColorSpace::ProPhotoRgb),
        ("color", Some("xyz" | "xyz-d65")) => Ok(ColorSpace::XyzD65),
        ("color", Some("xyz-d50")) => Ok(ColorSpace::XyzD50),
        _ => Err(ColorError::UnsupportedSyntax),
    }
}

fn variables(space: ColorSpace, kind: &str, color: Color) -> [(&'static str, f64); 4] {
    let mut values = color.channels();
    let names = match space {
        ColorSpace::Hsl => ["h", "s", "l", "alpha"],
        ColorSpace::Hwb => ["h", "w", "b", "alpha"],
        ColorSpace::Lab | ColorSpace::Oklab => ["l", "a", "b", "alpha"],
        ColorSpace::Lch | ColorSpace::Oklch => ["l", "c", "h", "alpha"],
        ColorSpace::XyzD50 | ColorSpace::XyzD65 => ["x", "y", "z", "alpha"],
        _ => ["r", "g", "b", "alpha"],
    };
    if kind == "rgb" || kind == "rgba" {
        for value in &mut values {
            *value *= 255.0;
        }
    }
    [
        (names[0], values[0]),
        (names[1], values[1]),
        (names[2], values[2]),
        (names[3], color.alpha()),
    ]
}

fn percent_reference(kind: &str, space: ColorSpace, index: usize) -> f64 {
    match space {
        ColorSpace::Srgb if kind != "color" => 255.0,
        ColorSpace::Hsl | ColorSpace::Hwb => 100.0,
        ColorSpace::Lab | ColorSpace::Lch => match index {
            0 => 100.0,
            1 => {
                if space == ColorSpace::Lch {
                    150.0
                } else {
                    125.0
                }
            }
            _ => 125.0,
        },
        ColorSpace::Oklab | ColorSpace::Oklch => {
            if index == 0 {
                1.0
            } else {
                0.4
            }
        }
        _ => 1.0,
    }
}

fn output_component(
    value: Quantity,
    kind: &str,
    space: ColorSpace,
    index: usize,
    relative: bool,
) -> Result<f64, ColorError> {
    let angle = match space {
        ColorSpace::Hsl | ColorSpace::Hwb => index == 0,
        ColorSpace::Lch | ColorSpace::Oklch => index == 2,
        _ => false,
    };
    if angle {
        return match value.unit {
            Unit::Number | Unit::Angle => Ok(value.value.rem_euclid(360.0)),
            Unit::Percent => Err(ColorError::InvalidSyntax),
        };
    }
    let raw = match value.unit {
        Unit::Percent => value.value * percent_reference(kind, space, index) / 100.0,
        Unit::Number => {
            if !relative && matches!(space, ColorSpace::Hsl | ColorSpace::Hwb) {
                return Err(ColorError::InvalidSyntax);
            }
            value.value
        }
        Unit::Angle => return Err(ColorError::InvalidSyntax),
    };
    if kind == "rgb" || kind == "rgba" {
        Ok(raw / 255.0)
    } else {
        Ok(raw)
    }
}

fn evaluate_channel(
    text: &str,
    variables: &[(&'static str, f64)],
) -> Result<Option<Quantity>, ColorError> {
    if text == "none" {
        return Ok(None);
    }
    if text.starts_with("var(") {
        return Err(ColorError::UnsupportedSyntax);
    }
    MathParser::new(text, variables).evaluate().map(Some)
}

/// Evaluate absolute calc() or relative colors into the opt-in missing-aware
/// representation. Origins may be nested up to 16 levels, without external
/// environment resolution. Numeric Color API is deliberately unchanged.
pub fn parse_css_expression(input: &str) -> Result<CssColor, ColorError> {
    parse_inner(input, 0)
}

fn parse_inner(input: &str, depth: usize) -> Result<CssColor, ColorError> {
    if depth > MAX_DEPTH {
        return Err(ColorError::UnsupportedSyntax);
    }
    let src = input.trim();
    if src.is_empty() || src.len() > MAX_CSS_INPUT || !src.is_ascii() {
        return Err(ColorError::InvalidSyntax);
    }
    let src = src.to_ascii_lowercase();
    let opening = src.find('(').ok_or(ColorError::InvalidSyntax)?;
    if !src.ends_with(')') {
        return Err(ColorError::InvalidSyntax);
    }
    let kind = src[..opening].trim();
    let fields = top_tokens(&src[opening + 1..src.len() - 1])?;
    let relative = fields.first() == Some(&"from");
    let mut index = usize::from(relative);
    let origin = if relative {
        let source = fields.get(index).ok_or(ColorError::InvalidSyntax)?;
        index += 1;
        let is_relative = source
            .split_once('(')
            .is_some_and(|(_, body)| body.trim_start().starts_with("from "));
        let inner = if source.contains("calc(") || is_relative {
            parse_inner(source, depth + 1)?
        } else {
            parse_css_missing(source)?
        };
        Some(inner)
    } else {
        None
    };
    let model = if kind == "color" {
        let name = *fields.get(index).ok_or(ColorError::InvalidSyntax)?;
        index += 1;
        Some(name)
    } else {
        None
    };
    let space = color_space(kind, model)?;
    let expected = index + 3;
    if fields.len() != expected && fields.len() != expected + 2 {
        return Err(ColorError::InvalidSyntax);
    }
    if fields.len() == expected + 2 && fields[expected] != "/" {
        return Err(ColorError::InvalidSyntax);
    }
    let original = origin
        .map(|value| value.convert_numeric(space))
        .transpose()?;
    let refs = original.map(|color| variables(space, kind, color));
    let env = refs.as_ref().map(|items| items.as_slice()).unwrap_or(&[]);
    let mut channels = [0.0_f64; 3];
    let mut missing = 0u8;
    for i in 0..3 {
        if let Some(value) = evaluate_channel(fields[index + i], env)? {
            channels[i] = output_component(value, kind, space, i, relative)?;
        } else {
            missing |= 1 << i;
        }
    }
    let alpha = if fields.len() == expected + 2 {
        match evaluate_channel(fields[expected + 1], env)? {
            Some(v) => match v.unit {
                Unit::Number => v.value.clamp(0.0, 1.0),
                Unit::Percent => (v.value / 100.0).clamp(0.0, 1.0),
                Unit::Angle => return Err(ColorError::InvalidSyntax),
            },
            None => {
                missing |= 8;
                0.0
            }
        }
    } else {
        original.map_or(1.0, Color::alpha)
    };
    CssColor::new(Color::new(space, channels, alpha)?, missing)
}
