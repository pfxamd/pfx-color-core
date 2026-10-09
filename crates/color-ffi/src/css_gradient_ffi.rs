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

/// # Safety
/// ptr may be NULL; otherwise must be a live handle allocated by
/// pfx_css_gradient_new, not previously freed.
#[no_mangle]
pub unsafe extern "C" fn pfx_css_gradient_free(ptr: *mut PfxCssGradient) {
    if !ptr.is_null() {
        drop(Box::from_raw(ptr));
    }
}
