//! Opt-in, pixel-coordinate CSS gradient builder. The existing normalized
//! pfx_gradient_* ABI and 40-byte PfxColor layout remain unchanged.
use super::{
    finish_color, from_wire, gamut_method, hue_method, space, PfxColor, COLOR, ENUM, LIMIT, NULL,
};
use pfx_color_core::{
    Color, ColorSpace, CssGradient, CssGradientKind, CssGradientOptions, CssRadialExtent,
    CssRadialShape, CssRadialSize, GradientStop,
};

pub struct PfxCssGradient {
    stops: Vec<GradientStop>,
    options: CssGradientOptions,
}

fn size(code: u32, radius_x: f64, radius_y: f64) -> Result<CssRadialSize, i32> {
    let extent = match code {
        0 => CssRadialExtent::ClosestSide,
        1 => CssRadialExtent::FarthestSide,
        2 => CssRadialExtent::ClosestCorner,
        3 => CssRadialExtent::FarthestCorner,
        4 => return Ok(CssRadialSize::Explicit { radius_x, radius_y }),
        _ => return Err(ENUM),
    };
    Ok(CssRadialSize::Extent(extent))
}

/// Create a gradient in CSS pixel coordinates, with optional repetition.
/// kind: 0 linear, 1 radial, 2 conic. radial shape: 0 circle, 1 ellipse.
/// extent: 0 closest-side, 1 farthest-side, 2 closest-corner,
/// 3 farthest-corner, 4 explicit radii in CSS pixels.
/// Center coordinates are CSS pixels, not normalized unit-square fractions.
#[no_mangle]
pub extern "C" fn pfx_css_gradient_new(
    kind: u32,
    width: f64,
    height: f64,
    angle: f64,
    center_x: f64,
    center_y: f64,
    shape: u32,
    extent: u32,
    radius_x: f64,
    radius_y: f64,
    repeating: u32,
    in_space: u32,
    target: u32,
    hue: u32,
    mapping: u32,
) -> *mut PfxCssGradient {
    let result = (|| {
        if repeating > 1 {
            return Err(ENUM);
        }
        let kind = match kind {
            0 => CssGradientKind::Linear {
                angle_degrees: angle,
            },
            1 => CssGradientKind::Radial {
                center_x,
                center_y,
                shape: match shape {
                    0 => CssRadialShape::Circle,
                    1 => CssRadialShape::Ellipse,
                    _ => return Err(ENUM),
                },
                size: size(extent, radius_x, radius_y)?,
            },
            2 => CssGradientKind::Conic {
                from_degrees: angle,
                center_x,
                center_y,
            },
            _ => return Err(ENUM),
        };
        let options = CssGradientOptions {
            width,
            height,
            kind,
            repeating: repeating == 1,
            interpolation_space: space(in_space)?,
            target_space: space(target)?,
            hue_method: hue_method(hue)?,
            gamut_map: gamut_method(mapping)?,
        };
        // Eagerly validate geometry before returning an owned handle.
        let dummy = Color::new(ColorSpace::Srgb, [0.0; 3], 1.0).map_err(|_| COLOR)?;
        let stops = [
            GradientStop {
                position: 0.0,
                color: dummy,
            },
            GradientStop {
                position: 1.0,
                color: dummy,
            },
        ];
        CssGradient::new(&stops, options).map_err(|_| COLOR)?;
        Ok(options)
    })();
    result
        .map(|options| {
            Box::into_raw(Box::new(PfxCssGradient {
                stops: Vec::new(),
                options,
            }))
        })
        .unwrap_or(std::ptr::null_mut())
}

/// # Safety
/// gradient must be an initialized live handle from pfx_css_gradient_new.
/// color must be a readable, aligned, initialized live PfxColor.
#[no_mangle]
pub unsafe extern "C" fn pfx_css_gradient_add_stop(
    gradient: *mut PfxCssGradient,
    position: f64,
    color: *const PfxColor,
) -> i32 {
    let result = (|| {
        let handle = gradient.as_mut().ok_or(NULL)?;
        if handle.stops.len() >= LIMIT || !position.is_finite() {
            return Err(COLOR);
        }
        let wire = *color.as_ref().ok_or(NULL)?;
        let color = from_wire(wire)?;
        handle.stops.push(GradientStop { position, color });
        Ok(0)
    })();
    result.unwrap_or_else(|error| error)
}

fn evaluated(handle: &PfxCssGradient) -> Result<CssGradient, i32> {
    CssGradient::new(&handle.stops, handle.options).map_err(|_| COLOR)
}

/// # Safety
/// gradient and out must be initialized, aligned, live pointers.
#[no_mangle]
pub unsafe extern "C" fn pfx_css_gradient_sample_pixel(
    gradient: *const PfxCssGradient,
    x: f64,
    y: f64,
    out: *mut PfxColor,
) -> i32 {
    let result = (|| {
        let handle = gradient.as_ref().ok_or(NULL)?;
        Ok(evaluated(handle)?
            .sample_pixel(x, y)
            .map_err(|_| COLOR)?
            .color)
    })();
    finish_color(result, out)
}

/// # Safety
/// gradient and out must be initialized, aligned, live pointers.
#[no_mangle]
pub unsafe extern "C" fn pfx_css_gradient_sample_progress(
    gradient: *const PfxCssGradient,
    position: f64,
    out: *mut PfxColor,
) -> i32 {
    let result = (|| {
        let handle = gradient.as_ref().ok_or(NULL)?;
        Ok(evaluated(handle)?
            .sample_progress(position)
            .map_err(|_| COLOR)?
            .color)
    })();
    finish_color(result, out)
}

/// Owned RGBA8 pixels in row-major order. Callers must not hold the
/// data pointer after free; WASM callers must copy before freeing.
pub struct PfxCssRaster {
    pixels: Vec<u8>,
}
const MAX_RASTER_PIXELS: usize = 1_048_576;

fn byte(value: f64) -> u8 {
    (value.clamp(0.0, 1.0) * 255.0).round() as u8
}

/// Render a CSS pixel gradient in one Rust call, rather than creating one
/// PfxColor and revalidating the gradient for every pixel. Pixel positions
/// are the centers of whole CSS px cells (x + 0.5, y + 0.5).
///
/// width/height must exactly equal the gradient's CSS pixel box dimensions.
/// Returns NULL on invalid input or any sample error; never partial pixels.
///
/// # Safety
/// gradient must be a live aligned handle from pfx_css_gradient_new.
#[no_mangle]
pub unsafe extern "C" fn pfx_css_gradient_raster_rgba8(
    gradient: *const PfxCssGradient,
    width: u32,
    height: u32,
) -> *mut PfxCssRaster {
    let result = (|| {
        let handle = gradient.as_ref().ok_or(NULL)?;
        let count = (width as usize).checked_mul(height as usize).ok_or(COLOR)?;
        if count == 0
            || count > MAX_RASTER_PIXELS
            || handle.options.width != width as f64
            || handle.options.height != height as f64
        {
            return Err(COLOR);
        }
        let gradient = evaluated(handle)?;
        let mut pixels = Vec::with_capacity(count * 4);
        for y in 0..height {
            for x in 0..width {
                let sample = gradient
                    .sample_pixel(x as f64 + 0.5, y as f64 + 0.5)
                    .map_err(|_| COLOR)?
                    .color;
                let color = if sample.space() == ColorSpace::Srgb {
                    sample
                } else {
                    sample.to(ColorSpace::Srgb).map_err(|_| COLOR)?
                };
                for channel in color.channels() {
                    pixels.push(byte(channel));
                }
                pixels.push(byte(color.alpha()));
            }
        }
        Ok(Box::into_raw(Box::new(PfxCssRaster { pixels })))
    })();
    result.unwrap_or(std::ptr::null_mut())
}

/// Approximate preview acceleration with a per-color-segment LUT. Hard
/// boundaries remain exact because the matching stop interval is chosen
/// before interpolating the LUT. This is intentionally separate from
/// pfx_css_gradient_raster_rgba8, which retains exact scalar semantics.
struct PreviewLut {
    stops: Vec<f64>,
    segments: Vec<Vec<[u8; 4]>>,
    first: [u8; 4],
    last: [u8; 4],
    constant: Option<[u8; 4]>,
    repeating: bool,
}

fn rgba(color: Color) -> Result<[u8; 4], i32> {
    let srgb = if color.space() == ColorSpace::Srgb {
        color
    } else {
        color.to(ColorSpace::Srgb).map_err(|_| COLOR)?
    };
    let rgb = srgb.channels();
    Ok([byte(rgb[0]), byte(rgb[1]), byte(rgb[2]), byte(srgb.alpha())])
}

impl PreviewLut {
    fn new(gradient: &CssGradient) -> Result<Self, i32> {
        let stops = gradient.stops();
        let positions: Vec<f64> = stops.iter().map(|s| s.position).collect();
        let first = rgba(gradient.sample_progress(stops[0].position - 1.0).map_err(|_| COLOR)?.color)?;
        let last = rgba(gradient.sample_progress(stops[stops.len()-1].position).map_err(|_| COLOR)?.color)?;
        let repeating = gradient.options().repeating;
        let constant = if repeating && positions[0] == positions[positions.len() - 1] {
            Some(rgba(gradient.sample_progress(positions[0]).map_err(|_| COLOR)?.color)?)
        } else {
            None
        };
        let bins = (8192 / (stops.len() - 1)).clamp(32, 1024);
        let mut segments = Vec::with_capacity(stops.len() - 1);
        for i in 0..stops.len() - 1 {
            let a = stops[i].position;
            let b = stops[i + 1].position;
            if b <= a {
                segments.push(Vec::new());
                continue;
            }
            let mut samples = Vec::with_capacity(bins + 1);
            for j in 0..=bins {
                // Never cross the right side of a hard stop at the final
                // sample: its left limit is the first color at that stop.
                let sampled = if j == bins {
                    rgba(gradient.sample_progress(b - (b - a) * 1e-12).map_err(|_| COLOR)?.color)?
                } else {
                    let fraction = j as f64 / bins as f64;
                    rgba(gradient.sample_progress(a + (b - a) * fraction).map_err(|_| COLOR)?.color)?
                };
                samples.push(sampled);
            }
            segments.push(samples);
        }
        Ok(Self { stops: positions, segments, first, last, constant, repeating })
    }

    fn sample(&self, mut progress: f64) -> Result<[u8; 4], i32> {
        if !progress.is_finite() { return Err(COLOR); }
        if let Some(value) = self.constant { return Ok(value); }
        let first = self.stops[0];
        let last = self.stops[self.stops.len() - 1];
        if self.repeating {
            progress = (progress - first).rem_euclid(last - first) + first;
            if !progress.is_finite() { return Err(COLOR); }
        }
        if progress < first { return Ok(self.first); }
        if progress >= last { return Ok(self.last); }
        let right = self.stops.partition_point(|stop| *stop <= progress);
        let index = right - 1;
        let ramp = &self.segments[index];
        if ramp.is_empty() { return Ok(self.last); }
        let t = ((progress - self.stops[index])
            / (self.stops[right] - self.stops[index]))
            * (ramp.len() - 1) as f64;
        let low = (t.floor() as usize).min(ramp.len() - 1);
        let high = (low + 1).min(ramp.len() - 1);
        let mix = (t - low as f64).clamp(0.0, 1.0);
        let mut pixel = [0; 4];
        for channel in 0..4 {
            pixel[channel] = ((ramp[low][channel] as f64 * (1.0 - mix))
                + (ramp[high][channel] as f64 * mix)).round() as u8;
        }
        Ok(pixel)
    }
}

/// Fast *preview* raster, quantized in each stop segment. Maximum 8192 LUT
/// samples, plus per-pixel geometry. Can differ by a few 8-bit units from the
/// exact scalar renderer; use pfx_css_gradient_raster_rgba8 for exact output.
///
/// # Safety
/// gradient must be a live aligned handle from pfx_css_gradient_new.
#[no_mangle]
pub unsafe extern "C" fn pfx_css_gradient_raster_preview_rgba8(
    gradient: *const PfxCssGradient,
    width: u32,
    height: u32,
) -> *mut PfxCssRaster {
    let result = (|| {
        let handle = gradient.as_ref().ok_or(NULL)?;
        let count = (width as usize).checked_mul(height as usize).ok_or(COLOR)?;
        if count == 0 || count > MAX_RASTER_PIXELS
            || handle.options.width != width as f64
            || handle.options.height != height as f64
        {
            return Err(COLOR);
        }
        let gradient = evaluated(handle)?;
        let lookup = PreviewLut::new(&gradient)?;
        let mut pixels = Vec::with_capacity(count * 4);
        for y in 0..height {
            for x in 0..width {
                let position = gradient.progress_at(x as f64 + 0.5, y as f64 + 0.5)
                    .map_err(|_| COLOR)?;
                pixels.extend_from_slice(&lookup.sample(position)?);
            }
        }
        Ok(Box::into_raw(Box::new(PfxCssRaster { pixels })))
    })();
    result.unwrap_or(std::ptr::null_mut())
}

/// # Safety
/// raster must be a readable, aligned, live handle allocated by
/// pfx_css_gradient_raster_rgba8, not previously freed.
#[no_mangle]
pub unsafe extern "C" fn pfx_css_raster_ptr(raster: *const PfxCssRaster) -> *const u8 {
    raster
        .as_ref()
        .map_or(std::ptr::null(), |image| image.pixels.as_ptr())
}

/// # Safety
/// raster must be a readable, aligned, live handle allocated by
/// pfx_css_gradient_raster_rgba8, not previously freed.
#[no_mangle]
pub unsafe extern "C" fn pfx_css_raster_len(raster: *const PfxCssRaster) -> u32 {
    raster.as_ref().map_or(0, |image| image.pixels.len() as u32)
}

/// # Safety
/// ptr may be NULL; otherwise it must be a live handle allocated by
/// pfx_css_gradient_raster_rgba8, released exactly once.
#[no_mangle]
pub unsafe extern "C" fn pfx_css_raster_free(ptr: *mut PfxCssRaster) {
    if !ptr.is_null() {
        drop(Box::from_raw(ptr));
    }
}

/// # Safety
/// ptr may be NULL; otherwise must be a live handle allocated by
/// pfx_css_gradient_new, not previously freed.
#[no_mangle]
pub unsafe extern "C" fn pfx_css_gradient_free(ptr: *mut PfxCssGradient) {
    if !ptr.is_null() {
        drop(Box::from_raw(ptr));
    }
}
