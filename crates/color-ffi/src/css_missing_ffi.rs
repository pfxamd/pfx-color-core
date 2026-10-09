//! Additive, version-1 compatible C/WASM CSS missing-component interface.
//! Separate 48-byte wire value avoids changing the existing 40-byte PfxColor.
use super::{from_wire, hue_method, space, to_wire, PfxColor, COLOR, ENUM, NULL};
use pfx_color_core::{format_css_missing, interpolate_css_missing, parse_css_missing, CssColor};

#[repr(C)]
#[derive(Clone, Copy)]
pub struct PfxCssColor {
    pub color: PfxColor,
    pub missing: u32,
    pub reserved: u32,
}
impl Default for PfxCssColor {
    fn default() -> Self {
        Self {
            color: PfxColor::default(),
            missing: 0,
            reserved: 0,
        }
    }
}
fn decode(wire: PfxCssColor) -> Result<CssColor, i32> {
    if wire.reserved != 0 || wire.missing > 15 {
        return Err(COLOR);
    }
    CssColor::new(from_wire(wire.color)?, wire.missing as u8).map_err(|_| COLOR)
}
fn encode(value: CssColor) -> PfxCssColor {
    PfxCssColor {
        color: to_wire(value.numeric()),
        missing: u32::from(value.missing_mask()),
        reserved: 0,
    }
}
unsafe fn read(ptr: *const PfxCssColor) -> Result<CssColor, i32> {
    decode(*ptr.as_ref().ok_or(NULL)?)
}
fn finish(result: Result<CssColor, i32>, out: *mut PfxCssColor) -> i32 {
    if out.is_null() {
        return NULL;
    }
    match result {
        Ok(value) => {
            unsafe {
                *out = encode(value);
            }
            0
        }
        Err(err) => err,
    }
}
#[no_mangle]
pub extern "C" fn pfx_css_missing_color_size() -> u32 {
    std::mem::size_of::<PfxCssColor>() as u32
}
#[no_mangle]
pub extern "C" fn pfx_css_missing_color_new() -> *mut PfxCssColor {
    Box::into_raw(Box::new(PfxCssColor::default()))
}
/// # Safety
/// ptr must be null or an owned live pointer from pfx_css_missing_color_new, freed once.
#[no_mangle]
pub unsafe extern "C" fn pfx_css_missing_color_free(ptr: *mut PfxCssColor) {
    if !ptr.is_null() {
        drop(Box::from_raw(ptr));
    }
}
/// # Safety
/// out must be live, aligned and writable. Accepts 0..15 missing mask.
#[no_mangle]
pub unsafe extern "C" fn pfx_css_missing_color_set(
    out: *mut PfxCssColor,
    space_id: u32,
    c0: f64,
    c1: f64,
    c2: f64,
    alpha: f64,
    missing: u32,
) -> i32 {
    if out.is_null() {
        return NULL;
    }
    let result = (|| {
        if missing > 15 {
            return Err(COLOR);
        }
        let numeric =
            pfx_color_core::Color::new(space(space_id)?, [c0, c1, c2], alpha).map_err(|_| COLOR)?;
        CssColor::new(numeric, missing as u8).map_err(|_| COLOR)
    })();
    finish(result, out)
}
/// # Safety
/// ptr must be a live aligned and initialized PfxCssColor.
#[no_mangle]
pub unsafe extern "C" fn pfx_css_missing_color_get_mask(ptr: *const PfxCssColor) -> u32 {
    ptr.as_ref().map_or(u32::MAX, |v| v.missing)
}
/// # Safety
/// ptr must be a live aligned and initialized PfxCssColor.
#[no_mangle]
pub unsafe extern "C" fn pfx_css_missing_color_get_space(ptr: *const PfxCssColor) -> u32 {
    ptr.as_ref().map_or(u32::MAX, |v| v.color.space)
}
/// # Safety
/// ptr must be a live aligned and initialized PfxCssColor.
#[no_mangle]
pub unsafe extern "C" fn pfx_css_missing_color_get_channel(
    ptr: *const PfxCssColor,
    index: u32,
) -> f64 {
    ptr.as_ref()
        .and_then(|v| v.color.channels.get(index as usize).copied())
        .unwrap_or(f64::NAN)
}
/// # Safety
/// ptr must be a live aligned and initialized PfxCssColor.
#[no_mangle]
pub unsafe extern "C" fn pfx_css_missing_color_get_alpha(ptr: *const PfxCssColor) -> f64 {
    ptr.as_ref().map_or(f64::NAN, |v| v.color.alpha)
}
/// # Safety
/// text must point to length live bytes and out to an initialized writable PfxCssColor.
#[no_mangle]
pub unsafe extern "C" fn pfx_css_missing_color_parse(
    text: *const u8,
    length: u32,
    out: *mut PfxCssColor,
) -> i32 {
    if text.is_null() || out.is_null() {
        return NULL;
    }
    if length == 0 || length as usize > pfx_color_core::css::MAX_CSS_INPUT {
        return COLOR;
    }
    let bytes = std::slice::from_raw_parts(text, length as usize);
    let result = std::str::from_utf8(bytes)
        .map_err(|_| COLOR)
        .and_then(|s| parse_css_missing(s).map_err(|_| COLOR));
    finish(result, out)
}
/// # Safety
/// color and output must be valid nonoverlapping live pointers;
/// output must have capacity bytes writable. Returns UTF-8 length excluding NUL or error.
#[no_mangle]
pub unsafe extern "C" fn pfx_css_missing_color_format(
    color: *const PfxCssColor,
    output: *mut u8,
    capacity: u32,
) -> i32 {
    if color.is_null() || output.is_null() {
        return NULL;
    }
    let text = match read(color).and_then(|c| format_css_missing(c).map_err(|_| COLOR)) {
        Ok(value) => value,
        Err(e) => return e,
    };
    if text.len() >= capacity as usize {
        return -5;
    }
    std::ptr::copy_nonoverlapping(text.as_ptr(), output, text.len());
    *output.add(text.len()) = 0;
    text.len() as i32
}
/// Ordinary CSS conversion consumes missing components as zero, and clears mask.
/// # Safety
/// input and output must be valid live pointers; may alias.
#[no_mangle]
pub unsafe extern "C" fn pfx_css_missing_color_convert(
    input: *const PfxCssColor,
    target: u32,
    out: *mut PfxCssColor,
) -> i32 {
    let result = (|| {
        let c = read(input)?;
        let numeric = c.convert_numeric(space(target)?).map_err(|_| COLOR)?;
        CssColor::new(numeric, 0).map_err(|_| COLOR)
    })();
    finish(result, out)
}
/// Interpolates with analogous-component carry and alpha borrowing.
/// # Safety
/// a, b and out must be valid live pointers; output may alias an input.
#[no_mangle]
pub unsafe extern "C" fn pfx_css_missing_color_interpolate(
    a: *const PfxCssColor,
    b: *const PfxCssColor,
    fraction: f64,
    target: u32,
    hue_id: u32,
    out: *mut PfxCssColor,
) -> i32 {
    let result = (|| {
        let hue = hue_method(hue_id).map_err(|_| ENUM)?;
        interpolate_css_missing(read(a)?, read(b)?, fraction, space(target)?, hue)
            .map_err(|_| COLOR)
    })();
    finish(result, out)
}
