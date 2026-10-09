//! Dependency-free *absolute* CSS color parsing and formatting.
//!
//! Supported: #rgb/#rgba/#rrggbb/#rrggbbaa, rgb()/rgba() modern and
//! comma-separated legacy, hsl()/hsla(), hwb(), lab(), lch(), oklab(),
//! oklch(), color(srgb|srgb-linear|display-p3|rec2020|xyz-d65|xyz-d50),
//! transparent and selected common color names.
//!
//! Deliberately reject CSS missing components (\`none\`), calc()/var(),
//! relative colors and other unsupported grammar instead of silently
//! misrepresenting them. This is not a complete CSS parser.
//! Reference: https://www.w3.org/TR/css-color-4/

use crate::gamut::{map_to_gamut, GamutMap};
use crate::spaces::{Color, ColorError, ColorSpace};

pub const MAX_CSS_INPUT: usize = 1024;

fn parse_number(text: &str) -> Result<f64, ColorError> {
    let value: f64 = text.parse().map_err(|_| ColorError::InvalidSyntax)?;
    if value.is_finite() {
        Ok(value)
    } else {
        Err(ColorError::InvalidSyntax)
    }
}

fn number_or_percent(text: &str, percent_reference: f64) -> Result<f64, ColorError> {
    if let Some(num) = text.strip_suffix('%') {
        Ok(parse_number(num)? * percent_reference / 100.0)
    } else {
        parse_number(text)
    }
}

fn required_percent(text: &str) -> Result<f64, ColorError> {
    let percent = text.strip_suffix('%').ok_or(ColorError::InvalidSyntax)?;
    parse_number(percent)
}

fn alpha_component(text: &str) -> Result<f64, ColorError> {
    Ok(number_or_percent(text, 1.0)?.clamp(0.0, 1.0))
}

fn angle_component(text: &str) -> Result<f64, ColorError> {
    let degrees = if let Some(value) = text.strip_suffix("deg") {
        parse_number(value)?
    } else if let Some(value) = text.strip_suffix("turn") {
        parse_number(value)? * 360.0
    } else if let Some(value) = text.strip_suffix("grad") {
        parse_number(value)? * 0.9
    } else if let Some(value) = text.strip_suffix("rad") {
        parse_number(value)? * (180.0 / std::f64::consts::PI)
    } else {
        parse_number(text)?
    };
    if degrees.is_finite() {
        Ok(degrees.rem_euclid(360.0))
    } else {
        Err(ColorError::InvalidSyntax)
    }
}

fn hex_digit_pair(text: &str) -> Result<u8, ColorError> {
    u8::from_str_radix(text, 16).map_err(|_| ColorError::InvalidSyntax)
}

fn parse_hex(input: &str) -> Result<Color, ColorError> {
    let hex = input.strip_prefix('#').ok_or(ColorError::InvalidSyntax)?;
    let rgba = match hex.len() {
        3 | 4 => {
            let mut components = [255_u8; 4];
            for (index, digit) in hex.as_bytes().iter().enumerate() {
                let byte = *digit as char;
                let n = byte.to_digit(16).ok_or(ColorError::InvalidSyntax)? as u8;
                components[index] = n * 17;
            }
            components
        }
        6 | 8 => {
            let mut components = [255_u8; 4];
            for (index, two) in hex.as_bytes().chunks(2).enumerate() {
                let text = std::str::from_utf8(two).map_err(|_| ColorError::InvalidSyntax)?;
                components[index] = hex_digit_pair(text)?;
            }
            components
        }
        _ => return Err(ColorError::InvalidSyntax),
    };
    Color::new(
        ColorSpace::Srgb,
        [
            rgba[0] as f64 / 255.0,
            rgba[1] as f64 / 255.0,
            rgba[2] as f64 / 255.0,
        ],
        rgba[3] as f64 / 255.0,
    )
}

fn parse_tokens(body: &str, legacy: bool) -> Result<([&str; 3], Option<&str>), ColorError> {
    if body.contains("none") || body.contains('(') || body.contains(')') {
        return Err(ColorError::UnsupportedSyntax);
    }
    if legacy {
        if body.contains('/') {
            return Err(ColorError::InvalidSyntax);
        }
        let parts: Vec<_> = body.split(',').map(str::trim).collect();
        if !(parts.len() == 3 || parts.len() == 4) || parts.iter().any(|s| s.is_empty()) {
            return Err(ColorError::InvalidSyntax);
        }
        Ok(([parts[0], parts[1], parts[2]], parts.get(3).copied()))
    } else {
        if body.contains(',') {
            return Err(ColorError::InvalidSyntax);
        }
        let (main, alpha) = if let Some((head, tail)) = body.split_once('/') {
            if tail.contains('/') {
                return Err(ColorError::InvalidSyntax);
            }
            let alpha = tail.trim();
            if alpha.split_whitespace().count() != 1 {
                return Err(ColorError::InvalidSyntax);
            }
            (head, Some(alpha))
        } else {
            (body, None)
        };
        let components: Vec<_> = main.split_whitespace().collect();
        if components.len() != 3 {
            return Err(ColorError::InvalidSyntax);
        }
        Ok(([components[0], components[1], components[2]], alpha))
    }
}

fn function(input: &str) -> Result<Color, ColorError> {
    let opening = input.find('(').ok_or(ColorError::InvalidSyntax)?;
    if !input.ends_with(')') {
        return Err(ColorError::InvalidSyntax);
    }
    let kind = input[..opening].trim();
    let body = input[opening + 1..input.len() - 1].trim();
    let legacy = body.contains(',');
    let (space, components, alpha) = if kind == "color" {
        if legacy || body.contains('(') || body.contains("none") {
            return Err(ColorError::UnsupportedSyntax);
        }
        let (head, alpha) = if let Some((head, tail)) = body.split_once('/') {
            if tail.contains('/') || tail.trim().split_whitespace().count() != 1 {
                return Err(ColorError::InvalidSyntax);
            }
            (head, Some(tail.trim()))
        } else {
            (body, None)
        };
        let parts: Vec<_> = head.split_whitespace().collect();
        if parts.len() != 4 {
            return Err(ColorError::InvalidSyntax);
        }
        let target = match parts[0] {
            "srgb" => ColorSpace::Srgb,
            "srgb-linear" => ColorSpace::SrgbLinear,
            "display-p3" => ColorSpace::DisplayP3,
            "rec2020" => ColorSpace::Rec2020,
            "xyz" | "xyz-d65" => ColorSpace::XyzD65,
            "xyz-d50" => ColorSpace::XyzD50,
            _ => return Err(ColorError::UnsupportedSyntax),
        };
        (target, [parts[1], parts[2], parts[3]], alpha)
    } else {
        let (channels, alpha) = parse_tokens(body, legacy)?;
        let space = match kind {
            "rgb" | "rgba" => ColorSpace::Srgb,
            "hsl" | "hsla" => ColorSpace::Hsl,
            "hwb" if !legacy => ColorSpace::Hwb,
            "lab" if !legacy => ColorSpace::Lab,
            "lch" if !legacy => ColorSpace::Lch,
            "oklab" if !legacy => ColorSpace::Oklab,
            "oklch" if !legacy => ColorSpace::Oklch,
            _ => return Err(ColorError::UnsupportedSyntax),
        };
        (space, channels, alpha)
    };
    let [a, b, c] = components;
    let channels = match space {
        ColorSpace::Srgb if kind == "rgb" || kind == "rgba" => {
            if legacy {
                let first_percent = a.ends_with('%');
                if b.ends_with('%') != first_percent || c.ends_with('%') != first_percent {
                    return Err(ColorError::InvalidSyntax);
                }
            }
            [
                number_or_percent(a, 255.0)? / 255.0,
                number_or_percent(b, 255.0)? / 255.0,
                number_or_percent(c, 255.0)? / 255.0,
            ]
        }
        ColorSpace::Hsl => [
            angle_component(a)?,
            required_percent(b)?,
            required_percent(c)?,
        ],
        ColorSpace::Hwb => [
            angle_component(a)?,
            required_percent(b)?,
            required_percent(c)?,
        ],
        ColorSpace::Lab => [
            number_or_percent(a, 100.0)?.clamp(0.0, 100.0),
            number_or_percent(b, 125.0)?,
            number_or_percent(c, 125.0)?,
        ],
        ColorSpace::Lch => [
            number_or_percent(a, 100.0)?.clamp(0.0, 100.0),
            number_or_percent(b, 150.0)?.max(0.0),
            angle_component(c)?,
        ],
        ColorSpace::Oklab => [
            number_or_percent(a, 1.0)?.clamp(0.0, 1.0),
            number_or_percent(b, 0.4)?,
            number_or_percent(c, 0.4)?,
        ],
        ColorSpace::Oklch => [
            number_or_percent(a, 1.0)?.clamp(0.0, 1.0),
            number_or_percent(b, 0.4)?.max(0.0),
            angle_component(c)?,
        ],
        _ => [
            number_or_percent(a, 1.0)?,
            number_or_percent(b, 1.0)?,
            number_or_percent(c, 1.0)?,
        ],
    };
    Color::new(
        space,
        channels,
        match alpha {
            Some(value) => alpha_component(value)?,
            None => 1.0,
        },
    )
}

fn named_color(input: &str) -> Option<&'static str> {
    match input {
        "black" => Some("#000"),
        "white" => Some("#fff"),
        "red" => Some("#f00"),
        "green" => Some("#008000"),
        "lime" => Some("#0f0"),
        "blue" => Some("#00f"),
        "yellow" => Some("#ff0"),
        "cyan" | "aqua" => Some("#0ff"),
        "magenta" | "fuchsia" => Some("#f0f"),
        "gray" | "grey" => Some("#808080"),
        "rebeccapurple" => Some("#663399"),
        "orange" => Some("#ffa500"),
        "purple" => Some("#800080"),
        "navy" => Some("#000080"),
        "teal" => Some("#008080"),
        "silver" => Some("#c0c0c0"),
        "maroon" => Some("#800000"),
        "olive" => Some("#808000"),
        "transparent" => Some("#0000"),
        _ => None,
    }
}

/// Parse an intentionally explicit subset of absolute CSS Color 4.
/// Unsupported valid CSS syntax is rejected, not approximated as black/none.
pub fn parse_css(input: &str) -> Result<Color, ColorError> {
    let trimmed = input.trim();
    if trimmed.is_empty() || trimmed.len() > MAX_CSS_INPUT || !trimmed.is_ascii() {
        return Err(ColorError::InvalidSyntax);
    }
    let lower = trimmed.to_ascii_lowercase();
    if lower.contains("none") {
        return Err(ColorError::UnsupportedSyntax);
    }
    if lower.starts_with('#') {
        return parse_hex(&lower);
    }
    if let Some(named) = named_color(&lower) {
        return parse_hex(named);
    }
    function(&lower)
}

fn with_alpha(body: String, alpha: f64) -> String {
    if alpha == 1.0 {
        body
    } else {
        format!("{body} / {alpha}")
    }
}

/// Format numeric colors as CSS absolute values, without silent gamut mapping.
/// The output uses sufficient decimal precision to round-trip its f64 channels.
/// CSS missing-component semantics are not representable in the numeric Color.
pub fn format_css(input: Color) -> Result<String, ColorError> {
    let [a, b, c] = input.channels();
    let alpha = input.alpha();
    Ok(match input.space() {
        ColorSpace::Srgb => format!(
            "rgb({})",
            with_alpha(format!("{} {} {}", a * 255.0, b * 255.0, c * 255.0), alpha)
        ),
        ColorSpace::Hsl => format!("hsl({})", with_alpha(format!("{a} {b}% {c}%"), alpha)),
        ColorSpace::Hwb => format!("hwb({})", with_alpha(format!("{a} {b}% {c}%"), alpha)),
        ColorSpace::Lab => format!("lab({})", with_alpha(format!("{a} {b} {c}"), alpha)),
        ColorSpace::Lch => format!("lch({})", with_alpha(format!("{a} {b} {c}"), alpha)),
        ColorSpace::Oklab => format!("oklab({})", with_alpha(format!("{a} {b} {c}"), alpha)),
        ColorSpace::Oklch => format!("oklch({})", with_alpha(format!("{a} {b} {c}"), alpha)),
        space => {
            let name = match space {
                ColorSpace::SrgbLinear => "srgb-linear",
                ColorSpace::DisplayP3 => "display-p3",
                ColorSpace::Rec2020 => "rec2020",
                ColorSpace::XyzD65 => "xyz-d65",
                ColorSpace::XyzD50 => "xyz-d50",
                // Nonstandard color spaces are converted to sRGB before
                // serialization, not claimed to have a CSS color() id.
                _ => return format_css(input.to(ColorSpace::Srgb)?),
            };
            format!(
                "color({name} {})",
                with_alpha(format!("{a} {b} {c}"), alpha)
            )
        }
    })
}

/// Encode hex with explicit gamut mapping policy. Opaque is 6 hex digits,
/// non-opaque is 8 digits. Convert to sRGB then quantize nearest 8-bit.
/// The original value remains unmodified.
pub fn format_hex(input: Color, mapping: GamutMap) -> Result<String, ColorError> {
    let mapped = map_to_gamut(input, ColorSpace::Srgb, mapping)?;
    let [r, g, b] = mapped.channels();
    let byte = |value: f64| -> u8 { (value.clamp(0.0, 1.0) * 255.0).round() as u8 };
    if input.alpha() == 1.0 {
        Ok(format!("#{:02x}{:02x}{:02x}", byte(r), byte(g), byte(b)))
    } else {
        Ok(format!(
            "#{:02x}{:02x}{:02x}{:02x}",
            byte(r),
            byte(g),
            byte(b),
            byte(input.alpha())
        ))
    }
}
