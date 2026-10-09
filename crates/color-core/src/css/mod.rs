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
            if tail.contains('/') || tail.split_whitespace().count() != 1 {
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

// CSS Color 4 canonical named-color RGB values (148 names).
// Data reviewed against colorjs/color-name (MIT), which documents W3C CSS values.
// See docs/third-party-notices.md for the source acknowledgement.
fn named_color(input: &str) -> Option<&'static str> {
    match input {
        "aliceblue" => Some("#f0f8ff"),
        "antiquewhite" => Some("#faebd7"),
        "aqua" => Some("#00ffff"),
        "aquamarine" => Some("#7fffd4"),
        "azure" => Some("#f0ffff"),
        "beige" => Some("#f5f5dc"),
        "bisque" => Some("#ffe4c4"),
        "black" => Some("#000000"),
        "blanchedalmond" => Some("#ffebcd"),
        "blue" => Some("#0000ff"),
        "blueviolet" => Some("#8a2be2"),
        "brown" => Some("#a52a2a"),
        "burlywood" => Some("#deb887"),
        "cadetblue" => Some("#5f9ea0"),
        "chartreuse" => Some("#7fff00"),
        "chocolate" => Some("#d2691e"),
        "coral" => Some("#ff7f50"),
        "cornflowerblue" => Some("#6495ed"),
        "cornsilk" => Some("#fff8dc"),
        "crimson" => Some("#dc143c"),
        "cyan" => Some("#00ffff"),
        "darkblue" => Some("#00008b"),
        "darkcyan" => Some("#008b8b"),
        "darkgoldenrod" => Some("#b8860b"),
        "darkgray" => Some("#a9a9a9"),
        "darkgreen" => Some("#006400"),
        "darkgrey" => Some("#a9a9a9"),
        "darkkhaki" => Some("#bdb76b"),
        "darkmagenta" => Some("#8b008b"),
        "darkolivegreen" => Some("#556b2f"),
        "darkorange" => Some("#ff8c00"),
        "darkorchid" => Some("#9932cc"),
        "darkred" => Some("#8b0000"),
        "darksalmon" => Some("#e9967a"),
        "darkseagreen" => Some("#8fbc8f"),
        "darkslateblue" => Some("#483d8b"),
        "darkslategray" => Some("#2f4f4f"),
        "darkslategrey" => Some("#2f4f4f"),
        "darkturquoise" => Some("#00ced1"),
        "darkviolet" => Some("#9400d3"),
        "deeppink" => Some("#ff1493"),
        "deepskyblue" => Some("#00bfff"),
        "dimgray" => Some("#696969"),
        "dimgrey" => Some("#696969"),
        "dodgerblue" => Some("#1e90ff"),
        "firebrick" => Some("#b22222"),
        "floralwhite" => Some("#fffaf0"),
        "forestgreen" => Some("#228b22"),
        "fuchsia" => Some("#ff00ff"),
        "gainsboro" => Some("#dcdcdc"),
        "ghostwhite" => Some("#f8f8ff"),
        "gold" => Some("#ffd700"),
        "goldenrod" => Some("#daa520"),
        "gray" => Some("#808080"),
        "green" => Some("#008000"),
        "greenyellow" => Some("#adff2f"),
        "grey" => Some("#808080"),
        "honeydew" => Some("#f0fff0"),
        "hotpink" => Some("#ff69b4"),
        "indianred" => Some("#cd5c5c"),
        "indigo" => Some("#4b0082"),
        "ivory" => Some("#fffff0"),
        "khaki" => Some("#f0e68c"),
        "lavender" => Some("#e6e6fa"),
        "lavenderblush" => Some("#fff0f5"),
        "lawngreen" => Some("#7cfc00"),
        "lemonchiffon" => Some("#fffacd"),
        "lightblue" => Some("#add8e6"),
        "lightcoral" => Some("#f08080"),
        "lightcyan" => Some("#e0ffff"),
        "lightgoldenrodyellow" => Some("#fafad2"),
        "lightgray" => Some("#d3d3d3"),
        "lightgreen" => Some("#90ee90"),
        "lightgrey" => Some("#d3d3d3"),
        "lightpink" => Some("#ffb6c1"),
        "lightsalmon" => Some("#ffa07a"),
        "lightseagreen" => Some("#20b2aa"),
        "lightskyblue" => Some("#87cefa"),
        "lightslategray" => Some("#778899"),
        "lightslategrey" => Some("#778899"),
        "lightsteelblue" => Some("#b0c4de"),
        "lightyellow" => Some("#ffffe0"),
        "lime" => Some("#00ff00"),
        "limegreen" => Some("#32cd32"),
        "linen" => Some("#faf0e6"),
        "magenta" => Some("#ff00ff"),
        "maroon" => Some("#800000"),
        "mediumaquamarine" => Some("#66cdaa"),
        "mediumblue" => Some("#0000cd"),
        "mediumorchid" => Some("#ba55d3"),
        "mediumpurple" => Some("#9370db"),
        "mediumseagreen" => Some("#3cb371"),
        "mediumslateblue" => Some("#7b68ee"),
        "mediumspringgreen" => Some("#00fa9a"),
        "mediumturquoise" => Some("#48d1cc"),
        "mediumvioletred" => Some("#c71585"),
        "midnightblue" => Some("#191970"),
        "mintcream" => Some("#f5fffa"),
        "mistyrose" => Some("#ffe4e1"),
        "moccasin" => Some("#ffe4b5"),
        "navajowhite" => Some("#ffdead"),
        "navy" => Some("#000080"),
        "oldlace" => Some("#fdf5e6"),
        "olive" => Some("#808000"),
        "olivedrab" => Some("#6b8e23"),
        "orange" => Some("#ffa500"),
        "orangered" => Some("#ff4500"),
        "orchid" => Some("#da70d6"),
        "palegoldenrod" => Some("#eee8aa"),
        "palegreen" => Some("#98fb98"),
        "paleturquoise" => Some("#afeeee"),
        "palevioletred" => Some("#db7093"),
        "papayawhip" => Some("#ffefd5"),
        "peachpuff" => Some("#ffdab9"),
        "peru" => Some("#cd853f"),
        "pink" => Some("#ffc0cb"),
        "plum" => Some("#dda0dd"),
        "powderblue" => Some("#b0e0e6"),
        "purple" => Some("#800080"),
        "rebeccapurple" => Some("#663399"),
        "red" => Some("#ff0000"),
        "rosybrown" => Some("#bc8f8f"),
        "royalblue" => Some("#4169e1"),
        "saddlebrown" => Some("#8b4513"),
        "salmon" => Some("#fa8072"),
        "sandybrown" => Some("#f4a460"),
        "seagreen" => Some("#2e8b57"),
        "seashell" => Some("#fff5ee"),
        "sienna" => Some("#a0522d"),
        "silver" => Some("#c0c0c0"),
        "skyblue" => Some("#87ceeb"),
        "slateblue" => Some("#6a5acd"),
        "slategray" => Some("#708090"),
        "slategrey" => Some("#708090"),
        "snow" => Some("#fffafa"),
        "springgreen" => Some("#00ff7f"),
        "steelblue" => Some("#4682b4"),
        "tan" => Some("#d2b48c"),
        "teal" => Some("#008080"),
        "thistle" => Some("#d8bfd8"),
        "tomato" => Some("#ff6347"),
        "turquoise" => Some("#40e0d0"),
        "violet" => Some("#ee82ee"),
        "wheat" => Some("#f5deb3"),
        "white" => Some("#ffffff"),
        "whitesmoke" => Some("#f5f5f5"),
        "yellow" => Some("#ffff00"),
        "yellowgreen" => Some("#9acd32"),
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
