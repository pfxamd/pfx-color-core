//! Stable versioned C ABI for the first-party PFx Color Core.
//!
//! This crate adds no third-party dependencies. It delegates all color
//! science calculations to the sibling pfx-color-core Rust crate.
//!
//! SAFETY: every non-null pointer crossing the ABI must refer to appropriately
//! aligned, initialized, live memory of its declared type. Handles must be
//! allocated by the matching pfx_*_new function, freed exactly once and never
//! shared between threads without caller-side synchronization.
//! C/WASM/JS callers must use ABI revision pfx_abi_version() == 1.

use pfx_color_core::{
    anchored_palette, contrast_ratio, difference, format_css, format_hex, generate_color_study,
    generate_custom_harmony, generate_harmony, interpolate, is_in_gamut, map_to_gamut, parse_css,
    ramp_palette, relative_luminance, tonal_palette, Color, ColorSpace, ColorStudy,
    ColorStudyOptions, DifferenceMethod, GamutMap, Gradient, GradientKind, GradientOptions,
    GradientStop, Harmony, HarmonyOptions, HarmonyScheme, HueMethod, Palette, RampOptions,
    TonalOptions,
};

const NULL: i32 = -1;
const ENUM: i32 = -2;
const COLOR: i32 = -3;
const INDEX: i32 = -4;
const LIMIT: usize = 256;

/// Color wire format. Exact layout: 4-byte space, 4-byte reserved,
/// 3 consecutive 8-byte coordinates, 8-byte alpha. Size: 40 bytes.
/// The caller must set reserved to zero. Space codes are versioned.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct PfxColor {
    pub space: u32,
    pub reserved: u32,
    pub channels: [f64; 3],
    pub alpha: f64,
}

impl Default for PfxColor {
    fn default() -> Self {
        Self {
            space: 0,
            reserved: 0,
            channels: [0.0; 3],
            alpha: 1.0,
        }
    }
}

fn space(code: u32) -> Result<ColorSpace, i32> {
    match code {
        0 => Ok(ColorSpace::Srgb),
        1 => Ok(ColorSpace::SrgbLinear),
        2 => Ok(ColorSpace::DisplayP3),
        3 => Ok(ColorSpace::DisplayP3Linear),
        4 => Ok(ColorSpace::Rec2020),
        5 => Ok(ColorSpace::Rec2020Linear),
        6 => Ok(ColorSpace::XyzD65),
        7 => Ok(ColorSpace::XyzD50),
        8 => Ok(ColorSpace::Lab),
        9 => Ok(ColorSpace::Lch),
        10 => Ok(ColorSpace::Oklab),
        11 => Ok(ColorSpace::Oklch),
        12 => Ok(ColorSpace::Hsl),
        13 => Ok(ColorSpace::Hwb),
        14 => Ok(ColorSpace::Hsv),
        15 => Ok(ColorSpace::A98Rgb),
        16 => Ok(ColorSpace::ProPhotoRgb),
        _ => Err(ENUM),
    }
}

fn difference_method(code: u32) -> Result<DifferenceMethod, i32> {
    match code {
        0 => Ok(DifferenceMethod::Cie76),
        1 => Ok(DifferenceMethod::Ciede2000),
        2 => Ok(DifferenceMethod::Ok),
        _ => Err(ENUM),
    }
}

fn gamut_method(code: u32) -> Result<GamutMap, i32> {
    match code {
        0 => Ok(GamutMap::Clip),
        1 => Ok(GamutMap::OklchChroma),
        2 => Ok(GamutMap::Css),
        _ => Err(ENUM),
    }
}

fn hue_method(code: u32) -> Result<HueMethod, i32> {
    match code {
        0 => Ok(HueMethod::Shorter),
        1 => Ok(HueMethod::Longer),
        2 => Ok(HueMethod::Increasing),
        3 => Ok(HueMethod::Decreasing),
        _ => Err(ENUM),
    }
}

fn harmony_scheme(code: u32) -> Result<HarmonyScheme, i32> {
    match code {
        0 => Ok(HarmonyScheme::Analogous),
        1 => Ok(HarmonyScheme::Complementary),
        2 => Ok(HarmonyScheme::SplitComplementary),
        3 => Ok(HarmonyScheme::Triadic),
        4 => Ok(HarmonyScheme::Tetradic),
        5 => Ok(HarmonyScheme::Square),
        _ => Err(ENUM),
    }
}

fn from_wire(wire: PfxColor) -> Result<Color, i32> {
    if wire.reserved != 0 {
        return Err(COLOR);
    }
    Color::new(space(wire.space)?, wire.channels, wire.alpha).map_err(|_| COLOR)
}

fn to_wire(color: Color) -> PfxColor {
    let space = match color.space() {
        ColorSpace::Srgb => 0,
        ColorSpace::SrgbLinear => 1,
        ColorSpace::DisplayP3 => 2,
        ColorSpace::DisplayP3Linear => 3,
        ColorSpace::Rec2020 => 4,
        ColorSpace::Rec2020Linear => 5,
        ColorSpace::XyzD65 => 6,
        ColorSpace::XyzD50 => 7,
        ColorSpace::Lab => 8,
        ColorSpace::Lch => 9,
        ColorSpace::Oklab => 10,
        ColorSpace::Oklch => 11,
        ColorSpace::Hsl => 12,
        ColorSpace::Hwb => 13,
        ColorSpace::Hsv => 14,
        ColorSpace::A98Rgb => 15,
        ColorSpace::ProPhotoRgb => 16,
    };
    PfxColor {
        space,
        reserved: 0,
        channels: color.channels(),
        alpha: color.alpha(),
    }
}

/// # Safety
/// ptr must be non-null, readable, aligned and point to a live PfxColor.
unsafe fn read(ptr: *const PfxColor) -> Result<Color, i32> {
    let wire = *ptr.as_ref().ok_or(NULL)?;
    from_wire(wire)
}

/// # Safety
/// ptr must be non-null, writable, aligned and point to a live PfxColor.
unsafe fn write(ptr: *mut PfxColor, color: Color) -> Result<(), i32> {
    *ptr.as_mut().ok_or(NULL)? = to_wire(color);
    Ok(())
}

/// # Safety
/// ptr must be a non-null, readable, aligned and live PfxColor pointer.
unsafe fn get_wire(ptr: *const PfxColor) -> Option<PfxColor> {
    ptr.as_ref().copied()
}

/// Returns 1. ABI changes require a new version and explicit compatibility.
#[no_mangle]
pub extern "C" fn pfx_abi_version() -> u32 {
    1
}

/// Returns sizeof(PfxColor), currently 40 bytes on native and WASM.
#[no_mangle]
pub extern "C" fn pfx_color_size() -> u32 {
    std::mem::size_of::<PfxColor>() as u32
}

/// Allocate an owned color buffer. Pass to pfx_color_free exactly once.
#[no_mangle]
pub extern "C" fn pfx_color_new() -> *mut PfxColor {
    Box::into_raw(Box::new(PfxColor::default()))
}

/// # Safety
/// ptr must be null or a pointer from pfx_color_new, not previously freed.
#[no_mangle]
pub unsafe extern "C" fn pfx_color_free(ptr: *mut PfxColor) {
    if !ptr.is_null() {
        drop(Box::from_raw(ptr));
    }
}

/// Initialize/update a live PfxColor. 0 success, negative for errors.
/// # Safety
/// ptr must be a non-null, writable, aligned, live PfxColor.
#[no_mangle]
pub unsafe extern "C" fn pfx_color_set(
    ptr: *mut PfxColor,
    space_code: u32,
    c0: f64,
    c1: f64,
    c2: f64,
    alpha: f64,
) -> i32 {
    let color = match Color::new(
        match space(space_code) {
            Ok(v) => v,
            Err(e) => return e,
        },
        [c0, c1, c2],
        alpha,
    ) {
        Ok(v) => v,
        Err(_) => return COLOR,
    };
    write(ptr, color).map(|_| 0).unwrap_or_else(|e| e)
}

/// Returns u32::MAX for null pointers.
/// # Safety
/// ptr must be null or a live PfxColor pointer.
#[no_mangle]
pub unsafe extern "C" fn pfx_color_get_space(ptr: *const PfxColor) -> u32 {
    get_wire(ptr).map_or(u32::MAX, |v| v.space)
}

/// Returns NaN for invalid index/null pointer.
/// # Safety
/// ptr must be null or a live PfxColor pointer.
#[no_mangle]
pub unsafe extern "C" fn pfx_color_get_channel(ptr: *const PfxColor, index: u32) -> f64 {
    get_wire(ptr)
        .and_then(|v| v.channels.get(index as usize).copied())
        .unwrap_or(f64::NAN)
}

/// Returns NaN for null pointer.
/// # Safety
/// ptr must be null or a live PfxColor pointer.
#[no_mangle]
pub unsafe extern "C" fn pfx_color_get_alpha(ptr: *const PfxColor) -> f64 {
    get_wire(ptr).map_or(f64::NAN, |v| v.alpha)
}

/// Convert without implicit clipping. Returns 0 or a negative error code.
/// # Safety
/// input and out must be valid PfxColor pointers (may point to the same value).
#[no_mangle]
pub unsafe extern "C" fn pfx_color_convert(
    input: *const PfxColor,
    target: u32,
    out: *mut PfxColor,
) -> i32 {
    let result = (|| read(input)?.to(space(target)?).map_err(|_| COLOR))();
    finish_color(result, out)
}

fn finish_color(result: Result<Color, i32>, out: *mut PfxColor) -> i32 {
    match result {
        Ok(value) => unsafe { write(out, value) }
            .map(|_| 0)
            .unwrap_or_else(|e| e),
        Err(e) => e,
    }
}

/// Return f64 NaN on error. Alpha ignored for color difference.
/// # Safety
/// Inputs must be valid PfxColor pointers.
#[no_mangle]
pub unsafe extern "C" fn pfx_color_difference(
    a: *const PfxColor,
    b: *const PfxColor,
    method: u32,
) -> f64 {
    (|| difference(read(a)?, read(b)?, difference_method(method)?).map_err(|_| COLOR))()
        .unwrap_or(f64::NAN)
}

/// Return f64 NaN on error, including transparent/out-of-gamut input.
/// # Safety
/// Inputs must be valid PfxColor pointers.
#[no_mangle]
pub unsafe extern "C" fn pfx_color_contrast(a: *const PfxColor, b: *const PfxColor) -> f64 {
    (|| contrast_ratio(read(a)?, read(b)?).map_err(|_| COLOR))().unwrap_or(f64::NAN)
}

/// Return f64 NaN on error.
/// # Safety
/// Input must be valid PfxColor pointer.
#[no_mangle]
pub unsafe extern "C" fn pfx_color_luminance(input: *const PfxColor) -> f64 {
    (|| relative_luminance(read(input)?).map_err(|_| COLOR))().unwrap_or(f64::NAN)
}

/// Returns 1 in gamut, 0 out of gamut, negative on invalid input.
/// # Safety
/// Input must be valid PfxColor pointer.
#[no_mangle]
pub unsafe extern "C" fn pfx_color_is_in_gamut(input: *const PfxColor, target: u32) -> i32 {
    match (|| is_in_gamut(read(input)?, space(target)?).map_err(|_| COLOR))() {
        Ok(v) => i32::from(v),
        Err(e) => e,
    }
}

/// Map with method 0=clip or 1=Oklch chroma.
/// # Safety
/// Input and out must be valid PfxColor pointers.
#[no_mangle]
pub unsafe extern "C" fn pfx_color_map(
    input: *const PfxColor,
    target: u32,
    method: u32,
    out: *mut PfxColor,
) -> i32 {
    let result =
        (|| map_to_gamut(read(input)?, space(target)?, gamut_method(method)?).map_err(|_| COLOR))();
    finish_color(result, out)
}

/// Fraction must be between 0 and 1. Four hue methods 0..3.
/// # Safety
/// Input and out pointers must point to valid live PfxColor values.
#[no_mangle]
pub unsafe extern "C" fn pfx_color_interpolate(
    a: *const PfxColor,
    b: *const PfxColor,
    fraction: f64,
    in_space: u32,
    hue: u32,
    out: *mut PfxColor,
) -> i32 {
    let result = (|| {
        interpolate(
            read(a)?,
            read(b)?,
            fraction,
            space(in_space)?,
            hue_method(hue)?,
        )
        .map_err(|_| COLOR)
    })();
    finish_color(result, out)
}

/// A heap-owned palette/harmony result, opaque to ABI callers.
pub struct PfxPalette {
    values: Vec<PfxPaletteEntry>,
}
struct PfxPaletteEntry {
    color: Color,
    position: f64,
    mapped: bool,
    hue_offset: f64,
}

fn wrap_palette(value: Palette) -> *mut PfxPalette {
    Box::into_raw(Box::new(PfxPalette {
        values: value
            .colors
            .into_iter()
            .map(|v| PfxPaletteEntry {
                color: v.color,
                position: v.position,
                mapped: v.mapped,
                hue_offset: 0.0,
            })
            .collect(),
    }))
}

/// Null on invalid input/parameters.
/// # Safety
/// seed must point to a valid PfxColor.
#[no_mangle]
pub unsafe extern "C" fn pfx_palette_tonal_new(
    seed: *const PfxColor,
    count: u32,
    min_l: f64,
    max_l: f64,
    chroma_scale: f64,
    target: u32,
    mapping: u32,
) -> *mut PfxPalette {
    let result = (|| {
        tonal_palette(
            read(seed)?,
            TonalOptions {
                count: count as usize,
                min_lightness: min_l,
                max_lightness: max_l,
                chroma_scale,
                target_space: space(target)?,
                gamut_map: gamut_method(mapping)?,
            },
        )
        .map_err(|_| COLOR)
    })();
    result.map(wrap_palette).unwrap_or(std::ptr::null_mut())
}

/// Null on invalid input/parameters.
/// # Safety
/// a and b must point to valid PfxColor values.
#[no_mangle]
pub unsafe extern "C" fn pfx_palette_ramp_new(
    a: *const PfxColor,
    b: *const PfxColor,
    count: u32,
    in_space: u32,
    target: u32,
    hue: u32,
    mapping: u32,
) -> *mut PfxPalette {
    let result = (|| {
        ramp_palette(
            read(a)?,
            read(b)?,
            RampOptions {
                count: count as usize,
                interpolation_space: space(in_space)?,
                target_space: space(target)?,
                hue_method: hue_method(hue)?,
                gamut_map: gamut_method(mapping)?,
            },
        )
        .map_err(|_| COLOR)
    })();
    result.map(wrap_palette).unwrap_or(std::ptr::null_mut())
}

/// Six schemes 0..5, with three configurable angles. Null on error.
/// # Safety
/// seed must point to a valid PfxColor.
#[no_mangle]
pub unsafe extern "C" fn pfx_palette_harmony_new(
    seed: *const PfxColor,
    scheme: u32,
    analogous: f64,
    split: f64,
    tetradic: f64,
    target: u32,
    mapping: u32,
) -> *mut PfxPalette {
    let result = (|| {
        generate_harmony(
            read(seed)?,
            harmony_scheme(scheme)?,
            HarmonyOptions {
                analogous_angle: analogous,
                split_angle: split,
                tetradic_angle: tetradic,
                target_space: space(target)?,
                gamut_map: gamut_method(mapping)?,
            },
        )
        .map_err(|_| COLOR)
    })();
    match result {
        Ok(v) => Box::into_raw(Box::new(PfxPalette {
            values: v
                .colors
                .into_iter()
                .map(|c| PfxPaletteEntry {
                    color: c.color,
                    position: c.index as f64,
                    mapped: c.mapped,
                    hue_offset: c.hue_offset,
                })
                .collect(),
        })),
        Err(_) => std::ptr::null_mut(),
    }
}

fn wrap_harmony(value: Harmony) -> *mut PfxPalette {
    Box::into_raw(Box::new(PfxPalette {
        values: value
            .colors
            .into_iter()
            .map(|entry| PfxPaletteEntry {
                color: entry.color,
                position: entry.index as f64,
                mapped: entry.mapped,
                hue_offset: entry.hue_offset,
            })
            .collect(),
    }))
}

/// Opaque owned anchor collection. Build with pfx_anchors_add, turn into a
/// palette with pfx_anchors_palette, then destroy with pfx_anchors_free.
pub struct PfxAnchors {
    colors: Vec<Color>,
}

/// Create an empty owned anchor builder. Add 2..=256 anchors before building.
#[no_mangle]
pub extern "C" fn pfx_anchors_new() -> *mut PfxAnchors {
    Box::into_raw(Box::new(PfxAnchors { colors: Vec::new() }))
}

/// Push an anchor. 0 success; negative status for null or invalid input.
/// The builder remains unchanged on error.
/// # Safety
/// builder must point to a live PfxAnchors from pfx_anchors_new;
/// color must point to a readable, live and aligned PfxColor.
#[no_mangle]
pub unsafe extern "C" fn pfx_anchors_add(builder: *mut PfxAnchors, color: *const PfxColor) -> i32 {
    let result = (|| {
        let value = read(color)?;
        let builder = builder.as_mut().ok_or(NULL)?;
        if builder.colors.len() >= LIMIT {
            return Err(COLOR);
        }
        builder.colors.push(value);
        Ok(0)
    })();
    result.unwrap_or_else(|error| error)
}

/// Generate a palette from 2..=256 inserted anchors. The original builder
/// remains alive, and every output palette owns an independent copy.
/// # Safety
/// builder must be a live pointer returned by pfx_anchors_new.
#[no_mangle]
pub unsafe extern "C" fn pfx_anchors_palette(
    builder: *const PfxAnchors,
    count: u32,
    interpolation_space: u32,
    target: u32,
    hue: u32,
    mapping: u32,
) -> *mut PfxPalette {
    let result = (|| {
        let builder = builder.as_ref().ok_or(NULL)?;
        anchored_palette(
            &builder.colors,
            RampOptions {
                count: count as usize,
                interpolation_space: space(interpolation_space)?,
                target_space: space(target)?,
                hue_method: hue_method(hue)?,
                gamut_map: gamut_method(mapping)?,
            },
        )
        .map_err(|_| COLOR)
    })();
    result.map(wrap_palette).unwrap_or(std::ptr::null_mut())
}

/// # Safety
/// builder must be null or a live pointer from pfx_anchors_new.
#[no_mangle]
pub unsafe extern "C" fn pfx_anchors_free(builder: *mut PfxAnchors) {
    if !builder.is_null() {
        drop(Box::from_raw(builder));
    }
}

/// Opaque owned collection of custom hue offsets, attached to a base color.
pub struct PfxCustomHarmony {
    seed: Color,
    offsets: Vec<f64>,
    options: HarmonyOptions,
}

/// Initialize a custom harmony builder. Returns null on invalid color or
/// target options. Add 2..=256 offsets, then generate a palette.
/// # Safety
/// seed must point to a live, readable PfxColor.
#[no_mangle]
pub unsafe extern "C" fn pfx_custom_harmony_new(
    seed: *const PfxColor,
    target: u32,
    mapping: u32,
) -> *mut PfxCustomHarmony {
    let result: Result<PfxCustomHarmony, i32> = (|| {
        Ok(PfxCustomHarmony {
            seed: read(seed)?,
            offsets: Vec::new(),
            options: HarmonyOptions {
                target_space: space(target)?,
                gamut_map: gamut_method(mapping)?,
                ..HarmonyOptions::default()
            },
        })
    })();
    result
        .map(|builder| Box::into_raw(Box::new(builder)))
        .unwrap_or(std::ptr::null_mut())
}

/// Append finite hue offset in degrees. No change on invalid input.
/// # Safety
/// builder must be a live pointer from pfx_custom_harmony_new.
#[no_mangle]
pub unsafe extern "C" fn pfx_custom_harmony_add(
    builder: *mut PfxCustomHarmony,
    offset_degrees: f64,
) -> i32 {
    let Some(builder) = builder.as_mut() else {
        return NULL;
    };
    if !offset_degrees.is_finite() || builder.offsets.len() >= LIMIT {
        return COLOR;
    }
    builder.offsets.push(offset_degrees);
    0
}

/// Generate owned custom harmony; builder is reusable after this call.
/// # Safety
/// builder must be a live pointer from pfx_custom_harmony_new.
#[no_mangle]
pub unsafe extern "C" fn pfx_custom_harmony_palette(
    builder: *const PfxCustomHarmony,
) -> *mut PfxPalette {
    let Some(builder) = builder.as_ref() else {
        return std::ptr::null_mut();
    };
    generate_custom_harmony(builder.seed, &builder.offsets, builder.options)
        .map(wrap_harmony)
        .unwrap_or(std::ptr::null_mut())
}

/// # Safety
/// builder must be null or a live pointer from pfx_custom_harmony_new.
#[no_mangle]
pub unsafe extern "C" fn pfx_custom_harmony_free(builder: *mut PfxCustomHarmony) {
    if !builder.is_null() {
        drop(Box::from_raw(builder));
    }
}

/// 0 for null. Palette handles must outlive any item read.
/// # Safety
/// ptr must be null or a live pointer from pfx_palette_*_new.
#[no_mangle]
pub unsafe extern "C" fn pfx_palette_len(ptr: *const PfxPalette) -> u32 {
    ptr.as_ref().map_or(0, |v| v.values.len() as u32)
}

/// Returns 0 for success, negative for invalid pointer/index.
/// # Safety
/// palette must be a live palette handle and out must be writable.
#[no_mangle]
pub unsafe extern "C" fn pfx_palette_get(
    palette: *const PfxPalette,
    index: u32,
    out: *mut PfxColor,
) -> i32 {
    let result = (|| {
        let entry = palette
            .as_ref()
            .ok_or(NULL)?
            .values
            .get(index as usize)
            .ok_or(INDEX)?;
        Ok(entry.color)
    })();
    finish_color(result, out)
}

/// Metadata: f64::NAN for invalid index/handle.
/// # Safety
/// palette must be null or a live palette handle.
#[no_mangle]
pub unsafe extern "C" fn pfx_palette_position(palette: *const PfxPalette, index: u32) -> f64 {
    palette
        .as_ref()
        .and_then(|v| v.values.get(index as usize))
        .map_or(f64::NAN, |e| e.position)
}

/// Metadata: 1 mapped, 0 unchanged, negative for invalid index/handle.
/// # Safety
/// palette must be null or a live palette handle.
#[no_mangle]
pub unsafe extern "C" fn pfx_palette_mapped(palette: *const PfxPalette, index: u32) -> i32 {
    palette
        .as_ref()
        .and_then(|v| v.values.get(index as usize))
        .map_or(INDEX, |e| i32::from(e.mapped))
}

/// Hue angle offset metadata; NAN on error.
/// # Safety
/// palette must be null or a live palette handle.
#[no_mangle]
pub unsafe extern "C" fn pfx_palette_hue_offset(palette: *const PfxPalette, index: u32) -> f64 {
    palette
        .as_ref()
        .and_then(|v| v.values.get(index as usize))
        .map_or(f64::NAN, |e| e.hue_offset)
}

/// # Safety
/// ptr must be null or an owned palette handle, not previously freed.
#[no_mangle]
pub unsafe extern "C" fn pfx_palette_free(ptr: *mut PfxPalette) {
    if !ptr.is_null() {
        drop(Box::from_raw(ptr));
    }
}

pub struct PfxGradient {
    stops: Vec<GradientStop>,
    options: GradientOptions,
}

/// Create an empty gradient builder. It requires 2+ stops before sampling.
/// kind 0=linear, 1=radial, 2=conic. Angle in degrees; center in [0,1].
/// Native ABI and WebAssembly wrapper share this implementation.
#[no_mangle]
pub extern "C" fn pfx_gradient_new(
    kind: u32,
    angle: f64,
    center_x: f64,
    center_y: f64,
    in_space: u32,
    target: u32,
    hue: u32,
    mapping: u32,
) -> *mut PfxGradient {
    let config = (|| {
        let kind = match kind {
            0 => GradientKind::Linear {
                angle_degrees: angle,
            },
            1 => GradientKind::Radial { center_x, center_y },
            2 => GradientKind::Conic {
                from_degrees: angle,
                center_x,
                center_y,
            },
            _ => return Err(ENUM),
        };
        Ok(GradientOptions {
            kind,
            interpolation_space: space(in_space)?,
            target_space: space(target)?,
            hue_method: hue_method(hue)?,
            gamut_map: gamut_method(mapping)?,
        })
    })();
    match config {
        Ok(options) => Box::into_raw(Box::new(PfxGradient {
            stops: Vec::new(),
            options,
        })),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Add stop preserving insertion order (including duplicates). 0 success.
/// # Safety
/// gradient must be a live gradient handle; color must be a valid PfxColor.
#[no_mangle]
pub unsafe extern "C" fn pfx_gradient_add_stop(
    gradient: *mut PfxGradient,
    position: f64,
    color: *const PfxColor,
) -> i32 {
    let result = (|| {
        let gradient = gradient.as_mut().ok_or(NULL)?;
        if gradient.stops.len() >= LIMIT
            || !position.is_finite()
            || !(0.0..=1.0).contains(&position)
        {
            return Err(COLOR);
        }
        let color = read(color)?;
        gradient.stops.push(GradientStop { position, color });
        Ok(0)
    })();
    result.unwrap_or_else(|e| e)
}

/// 0 success, negative on error, including insufficient stops.
/// # Safety
/// gradient must be a live gradient handle; out must be writable.
#[no_mangle]
pub unsafe extern "C" fn pfx_gradient_sample(
    gradient: *const PfxGradient,
    position: f64,
    out: *mut PfxColor,
) -> i32 {
    let result = (|| {
        let builder = gradient.as_ref().ok_or(NULL)?;
        let value = Gradient::new(&builder.stops, builder.options).map_err(|_| COLOR)?;
        Ok(value.sample(position).map_err(|_| COLOR)?.color)
    })();
    finish_color(result, out)
}

/// 0 success, negative on error, including insufficient stops.
/// # Safety
/// gradient must be a live gradient handle; out must be writable.
#[no_mangle]
pub unsafe extern "C" fn pfx_gradient_sample_xy(
    gradient: *const PfxGradient,
    x: f64,
    y: f64,
    out: *mut PfxColor,
) -> i32 {
    let result = (|| {
        let builder = gradient.as_ref().ok_or(NULL)?;
        let value = Gradient::new(&builder.stops, builder.options).map_err(|_| COLOR)?;
        Ok(value.sample_xy(x, y).map_err(|_| COLOR)?.color)
    })();
    finish_color(result, out)
}

/// # Safety
/// ptr must be null or an owned gradient handle, not previously freed.
#[no_mangle]
pub unsafe extern "C" fn pfx_gradient_free(ptr: *mut PfxGradient) {
    if !ptr.is_null() {
        drop(Box::from_raw(ptr));
    }
}

/// Allocate a byte buffer owned by Rust for input/output across the WASM ABI.
/// The caller MUST release it with pfx_buffer_free(ptr, SAME_SIZE).
/// Returns null for zero, oversized or invalid size.
#[no_mangle]
pub extern "C" fn pfx_buffer_new(length: u32) -> *mut u8 {
    if length == 0 || length > 2048 {
        return std::ptr::null_mut();
    }
    Box::into_raw(vec![0_u8; length as usize].into_boxed_slice()) as *mut u8
}

/// # Safety
/// ptr must be null, or an alive buffer from pfx_buffer_new(length), with
/// exactly the original length. Never free twice.
#[no_mangle]
pub unsafe extern "C" fn pfx_buffer_free(ptr: *mut u8, length: u32) {
    if !ptr.is_null() {
        if length == 0 || length > 2048 {
            return;
        }
        drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(
            ptr,
            length as usize,
        )));
    }
}

/// Parse a UTF-8 absolute CSS color supported by the standalone Rust core.
/// Returns 0 on success; negative on error, without writing to output.
/// # Safety
/// data must be a live, readable byte pointer to length bytes.
/// out must be a live, writable PfxColor pointer.
#[no_mangle]
pub unsafe extern "C" fn pfx_css_parse(data: *const u8, length: u32, out: *mut PfxColor) -> i32 {
    if data.is_null() || out.is_null() {
        return NULL;
    }
    if length == 0 || length as usize > pfx_color_core::css::MAX_CSS_INPUT {
        return COLOR;
    }
    let bytes = std::slice::from_raw_parts(data, length as usize);
    let result = std::str::from_utf8(bytes)
        .map_err(|_| COLOR)
        .and_then(|css| parse_css(css).map_err(|_| COLOR));
    finish_color(result, out)
}

/// Encode CSS text or 8-bit hex into a caller-owned UTF-8 buffer.
/// Format 0=CSS coordinates (never maps gamut), 1=hex (explicit map method).
/// On success, returns byte count EXCLUDING trailing NUL (a nonnegative i32).
/// Buffer MUST have at least the returned count + 1 bytes.
/// Returns -5 if capacity is inadequate, -1 on null, -3 on invalid input.
/// No truncation is performed.
/// # Safety
/// color must be a live PfxColor pointer; buffer must be writable for
/// capacity bytes and must not overlap the input PfxColor memory.
#[no_mangle]
pub unsafe extern "C" fn pfx_css_format(
    color: *const PfxColor,
    format_kind: u32,
    gamut_method_id: u32,
    buffer: *mut u8,
    capacity: u32,
) -> i32 {
    if color.is_null() || buffer.is_null() {
        return NULL;
    }
    let color = match read(color) {
        Ok(c) => c,
        Err(e) => return e,
    };
    let output = match format_kind {
        0 => match format_css(color) {
            Ok(s) => s,
            Err(_) => return COLOR,
        },
        1 => {
            let method = match gamut_method(gamut_method_id) {
                Ok(m) => m,
                Err(e) => return e,
            };
            match format_hex(color, method) {
                Ok(s) => s,
                Err(_) => return COLOR,
            }
        }
        _ => return ENUM,
    };
    if output.len() >= capacity as usize {
        return -5;
    }
    std::ptr::copy_nonoverlapping(output.as_ptr(), buffer, output.len());
    *buffer.add(output.len()) = 0;
    output.len() as i32
}

/// Heap-owned deterministic 10-color study.
pub struct PfxStudy {
    study: ColorStudy,
}

/// Return null on invalid arguments. Controls use a 0..100 range.
/// # Safety
/// seed must point to a live, initialized PfxColor value.
#[no_mangle]
pub unsafe extern "C" fn pfx_study_new(
    seed: *const PfxColor,
    rnd_seed: u32,
    lightness: f64,
    chroma: f64,
    hue_range: f64,
    tone_range: f64,
    target: u32,
    mapping: u32,
) -> *mut PfxStudy {
    let result = (|| {
        generate_color_study(
            read(seed)?,
            ColorStudyOptions {
                lightness,
                chroma,
                hue_range,
                tone_range,
                random_seed: rnd_seed,
                target_space: space(target)?,
                gamut_map: gamut_method(mapping)?,
            },
        )
        .map_err(|_| COLOR)
    })();
    result
        .map(|study| Box::into_raw(Box::new(PfxStudy { study })))
        .unwrap_or(std::ptr::null_mut())
}

/// # Safety
/// ptr must be null or a live study handle.
#[no_mangle]
pub unsafe extern "C" fn pfx_study_len(ptr: *const PfxStudy) -> u32 {
    ptr.as_ref().map_or(0, |v| v.study.colors.len() as u32)
}

/// Return scheme code 0..5, or u32::MAX on error.
/// # Safety
/// ptr must be null or a live study handle.
#[no_mangle]
pub unsafe extern "C" fn pfx_study_scheme(ptr: *const PfxStudy) -> u32 {
    ptr.as_ref().map_or(u32::MAX, |v| match v.study.scheme {
        HarmonyScheme::Analogous => 0,
        HarmonyScheme::Complementary => 1,
        HarmonyScheme::SplitComplementary => 2,
        HarmonyScheme::Triadic => 3,
        HarmonyScheme::Tetradic => 4,
        HarmonyScheme::Square => 5,
        HarmonyScheme::Custom => u32::MAX,
    })
}

/// Returns 0 success, negative for invalid pointers/index.
/// # Safety
/// study must be a live handle and out a writable PfxColor.
#[no_mangle]
pub unsafe extern "C" fn pfx_study_get(
    study: *const PfxStudy,
    index: u32,
    out: *mut PfxColor,
) -> i32 {
    let result = (|| {
        Ok(study
            .as_ref()
            .ok_or(NULL)?
            .study
            .colors
            .get(index as usize)
            .ok_or(INDEX)?
            .color)
    })();
    finish_color(result, out)
}

/// Returns 1 if mapped, 0 otherwise or negative error for missing item.
/// # Safety
/// study must be null or a live handle.
#[no_mangle]
pub unsafe extern "C" fn pfx_study_mapped(study: *const PfxStudy, index: u32) -> i32 {
    study
        .as_ref()
        .and_then(|s| s.study.colors.get(index as usize))
        .map_or(INDEX, |color| i32::from(color.mapped))
}

/// Already computed Oklch coordinate 0=L, 1=C, 2=h; NaN on error.
/// # Safety
/// study must be null or a live handle.
#[no_mangle]
pub unsafe extern "C" fn pfx_study_oklch(
    study: *const PfxStudy,
    index: u32,
    coordinate: u32,
) -> f64 {
    study
        .as_ref()
        .and_then(|s| s.study.colors.get(index as usize))
        .and_then(|c| c.oklch.get(coordinate as usize))
        .copied()
        .unwrap_or(f64::NAN)
}

/// # Safety
/// ptr must be null or an owned handle from pfx_study_new, not yet freed.
#[no_mangle]
pub unsafe extern "C" fn pfx_study_free(ptr: *mut PfxStudy) {
    if !ptr.is_null() {
        drop(Box::from_raw(ptr));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_color_space_codes_are_reversible() {
        for code in 0..17 {
            let kind = space(code).unwrap();
            let c = Color::new(kind, [0.5, 0.25, 0.8], 0.5).unwrap();
            assert_eq!(to_wire(c).space, code);
            assert_eq!(from_wire(to_wire(c)).unwrap(), c);
        }
        assert_eq!(space(15), Err(ENUM));
    }

    #[test]
    fn wire_layout_and_abi_are_explicit() {
        assert_eq!(pfx_color_size(), 40);
        assert_eq!(std::mem::align_of::<PfxColor>(), 8);
        assert_eq!(pfx_abi_version(), 1);
    }

    #[test]
    fn convert_and_difference_do_not_require_js_or_other_libraries() {
        let mut a = PfxColor::default();
        let mut b = PfxColor::default();
        let mut output = PfxColor::default();
        unsafe {
            assert_eq!(pfx_color_set(&mut a, 0, 0.0, 0.0, 0.0, 1.0), 0);
            assert_eq!(pfx_color_set(&mut b, 0, 1.0, 1.0, 1.0, 1.0), 0);
            assert_eq!(pfx_color_convert(&a, 11, &mut output), 0);
            assert_eq!(output.space, 11);
            assert!((pfx_color_contrast(&a, &b) - 21.0).abs() < 1e-12);
            assert!(pfx_color_difference(&a, &b, 2).is_finite());
        }
    }

    #[test]
    fn invalid_ffi_arguments_are_reported() {
        let mut out = PfxColor::default();
        unsafe {
            assert_eq!(pfx_color_convert(std::ptr::null(), 0, &mut out), NULL);
            assert_eq!(pfx_color_convert(&out, 888, &mut out), ENUM);
            assert_eq!(pfx_color_set(&mut out, 0, f64::NAN, 0.0, 0.0, 1.0), COLOR);
            assert!(pfx_color_luminance(std::ptr::null()).is_nan());
            assert!(pfx_palette_tonal_new(std::ptr::null(), 9, 0.1, 0.9, 1.0, 0, 1).is_null());
            assert!(pfx_gradient_new(333, 0.0, 0.5, 0.5, 11, 0, 0, 1).is_null());
        }
    }
}
