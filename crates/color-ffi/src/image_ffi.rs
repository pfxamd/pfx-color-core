//! C and WebAssembly interface for decoded RGBA8 image palette extraction.
//! The caller decodes pixels; the returned palette owns the result, not pixels.

use pfx_color_core::{
    extract_image_palette, ImagePalette, ImagePaletteOptions, ImageRegion,
    MAX_IMAGE_PALETTE_COLORS, MAX_IMAGE_SAMPLES,
};
use crate::{write, PfxColor, INDEX, NULL};

const MAX_RGBA_BYTES: usize = 64 * 1024 * 1024;

pub struct PfxImagePalette {
    result: ImagePalette,
}

/// Extract a palette from row-major, unpremultiplied, decoded RGBA8 data.
/// Region width=height=0 selects the entire image. Null means invalid input.
/// The result owns all data; the caller may release input pixels immediately.
/// # Safety
/// The pixel pointer must address length initialized bytes with valid alignment.
#[no_mangle]
pub unsafe extern "C" fn pfx_image_new(
    pixels: *const u8,
    length: u32,
    width: u32,
    height: u32,
    count: u32,
    stride: u32,
    max_samples: u32,
    alpha_threshold: u32,
    ignore_near_white: u32,
    region_x: u32,
    region_y: u32,
    region_width: u32,
    region_height: u32,
) -> *mut PfxImagePalette {
    let result = (|| {
        let bytes = (width as usize)
            .checked_mul(height as usize)?
            .checked_mul(4)?;
        if bytes == 0
            || bytes > MAX_RGBA_BYTES
            || bytes != length as usize
            || pixels.is_null()
            || count == 0
            || count as usize > MAX_IMAGE_PALETTE_COLORS
            || stride == 0
            || max_samples == 0
            || max_samples as usize > MAX_IMAGE_SAMPLES
            || alpha_threshold > 255
            || ignore_near_white > 1
        {
            return None;
        }
        let region = match (region_width, region_height) {
            (0, 0) => {
                if region_x != 0 || region_y != 0 {
                    return None;
                }
                None
            }
            (0, _) | (_, 0) => return None,
            (w, h) => Some(ImageRegion {
                x: region_x as usize,
                y: region_y as usize,
                width: w as usize,
                height: h as usize,
            }),
        };
        // The caller guarantees the memory described by length is live.
        let rgba = std::slice::from_raw_parts(pixels, bytes);
        extract_image_palette(
            rgba,
            width as usize,
            height as usize,
            ImagePaletteOptions {
                count: count as usize,
                stride: stride as usize,
                max_samples: max_samples as usize,
                alpha_threshold: alpha_threshold as u8,
                ignore_near_white: ignore_near_white == 1,
                region,
            },
        ).ok()
    })();
    result
        .map(|result| Box::into_raw(Box::new(PfxImagePalette { result })))
        .unwrap_or(std::ptr::null_mut())
}

/// # Safety
/// Input must be null or a valid live owned image handle.
#[no_mangle]
pub unsafe extern "C" fn pfx_image_len(ptr: *const PfxImagePalette) -> u32 {
    ptr.as_ref().map_or(0, |h| h.result.colors.len() as u32)
}

/// # Safety
/// Input must be null or a valid live owned image handle.
#[no_mangle]
pub unsafe extern "C" fn pfx_image_sampled(ptr: *const PfxImagePalette) -> u32 {
    ptr.as_ref().map_or(0, |h| h.result.sampled_pixels as u32)
}

/// # Safety
/// Input must be null or a valid live owned image handle.
#[no_mangle]
pub unsafe extern "C" fn pfx_image_eligible(ptr: *const PfxImagePalette) -> u32 {
    ptr.as_ref().map_or(0, |h| h.result.eligible_pixels as u32)
}

/// # Safety
/// Handle and output must be valid, live, aligned and nonaliasing.
#[no_mangle]
pub unsafe extern "C" fn pfx_image_get(
    ptr: *const PfxImagePalette,
    index: u32,
    out: *mut PfxColor,
) -> i32 {
    let Some(handle) = ptr.as_ref() else {
        return NULL;
    };
    let Some(item) = handle.result.colors.get(index as usize) else {
        return INDEX;
    };
    match write(out, item.color) {
        Ok(()) => 0,
        Err(error) => error,
    }
}

/// Population or a negative index error.
/// # Safety
/// Input must be null or a valid live image handle.
#[no_mangle]
pub unsafe extern "C" fn pfx_image_population(
    ptr: *const PfxImagePalette,
    index: u32,
) -> i32 {
    ptr.as_ref()
        .and_then(|h| h.result.colors.get(index as usize))
        .map_or(INDEX, |item| item.population as i32)
}

/// Population share in [0,1], NaN on invalid index.
/// # Safety
/// Input must be null or a valid live image handle.
#[no_mangle]
pub unsafe extern "C" fn pfx_image_proportion(
    ptr: *const PfxImagePalette,
    index: u32,
) -> f64 {
    ptr.as_ref()
        .and_then(|h| h.result.colors.get(index as usize))
        .map_or(f64::NAN, |item| item.proportion)
}

/// # Safety
/// Pointer must be null or an owned handle created by pfx_image_new, freed once.
#[no_mangle]
pub unsafe extern "C" fn pfx_image_free(ptr: *mut PfxImagePalette) {
    if !ptr.is_null() {
        drop(Box::from_raw(ptr));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_image_ffi_round_trip() {
        let bytes = [255, 0, 0, 255, 255, 0, 0, 255, 0, 0, 255, 255, 255, 0, 0, 255];
        let handle = unsafe {
            pfx_image_new(bytes.as_ptr(), 16, 2, 2, 2, 1, 100, 128, 0, 0, 0, 0, 0)
        };
        assert!(!handle.is_null());
        unsafe {
            assert_eq!(pfx_image_len(handle), 2);
            assert_eq!(pfx_image_sampled(handle), 4);
            assert_eq!(pfx_image_eligible(handle), 4);
            assert_eq!(pfx_image_population(handle, 0), 3);
            assert!((pfx_image_proportion(handle, 0) - 0.75).abs() < 1e-12);
            let mut out = PfxColor::default();
            assert_eq!(pfx_image_get(handle, 0, &mut out), 0);
            assert_eq!(out.channels, [1.0, 0.0, 0.0]);
            assert_eq!(pfx_image_get(handle, 3, &mut out), INDEX);
            pfx_image_free(handle);
        }
    }

    #[test]
    fn native_image_ffi_rejects_invalid_parameters() {
        let bytes = [255_u8, 0, 0, 255];
        unsafe {
            assert!(pfx_image_new(bytes.as_ptr(), 3, 1, 1, 2, 1,
                100, 128, 0, 0, 0, 0, 0).is_null());
            assert!(pfx_image_new(std::ptr::null(), 4, 1, 1, 2, 1,
                100, 128, 0, 0, 0, 0, 0).is_null());
            assert!(pfx_image_new(bytes.as_ptr(), 4, 1, 1, 2, 1,
                100, 128, 0, 0, 0, 1, 0).is_null());
            assert_eq!(pfx_image_len(std::ptr::null()), 0);
            assert_eq!(pfx_image_population(std::ptr::null(), 0), INDEX);
            assert!(pfx_image_proportion(std::ptr::null(), 0).is_nan());
        }
    }
}
