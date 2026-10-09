//! First-party, deterministic extraction of palettes from decoded RGBA8 images.
//!
//! This module accepts **raw pixels**, not PNG/JPEG/WEBP encoded bytes. Image
//! decoding belongs to the host (browser canvas, platform image API, etc.).
//! Sampling is bounded; alpha filtering precedes quantization. Clustering is
//! done in perceptual Oklab, with RGB centroids computed from the source
//! pixels to guarantee representative colors inside the sRGB gamut.
//!
//! No random initial states, filesystem, browser APIs, third-party crates or
//! mutable global state. Output is deterministic for the same pixel buffer.

use std::collections::BTreeMap;
use std::fmt;

use crate::spaces::{Color, ColorError, ColorSpace};

pub const MAX_IMAGE_PALETTE_COLORS: usize = 32;
pub const MAX_IMAGE_SAMPLES: usize = 500_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageError {
    InvalidDimensions,
    InvalidBuffer,
    InvalidRegion,
    InvalidCount,
    InvalidSampling,
    EmptyImage,
    Color(ColorError),
}

impl fmt::Display for ImageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for ImageError {}

impl From<ColorError> for ImageError {
    fn from(error: ColorError) -> Self {
        Self::Color(error)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImageRegion {
    pub x: usize,
    pub y: usize,
    pub width: usize,
    pub height: usize,
}

/// Configuration for deterministic, bounded image-palette extraction.
///
/// count is in 1..=32. stride >= 1 is a spatial subsampling step and the
/// maximum eligible image samples is in 1..=500_000. Every accepted RGBA
/// pixel contributes equally after the specified alpha threshold. Pixels
/// that are nearly white may be excluded (all channels >= 245).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImagePaletteOptions {
    pub count: usize,
    pub stride: usize,
    pub max_samples: usize,
    pub alpha_threshold: u8,
    pub ignore_near_white: bool,
    pub region: Option<ImageRegion>,
}

impl Default for ImagePaletteOptions {
    fn default() -> Self {
        Self {
            count: 6,
            stride: 1,
            max_samples: 80_000,
            alpha_threshold: 128,
            ignore_near_white: false,
            region: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ExtractedColor {
    /// Opaque, normalized sRGB, averaged from original pixels in this cluster.
    pub color: Color,
    /// Number of retained, spatially sampled pixels in this cluster.
    pub population: usize,
    /// Population divided by the total eligible (nonfiltered) sampled pixels.
    pub proportion: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImagePalette {
    pub colors: Vec<ExtractedColor>,
    /// Number of spatially visited pixels, including rejected/transparent ones.
    pub sampled_pixels: usize,
    /// Number of samples passing transparency and white filters.
    pub eligible_pixels: usize,
}

#[derive(Debug, Clone, Copy, Default)]
struct Cell {
    count: usize,
    rgb_sums: [u64; 3],
}

#[derive(Debug, Clone, Copy)]
struct Point {
    count: usize,
    oklab: [f64; 3],
    rgb: [f64; 3],
}

fn distance2(a: [f64; 3], b: [f64; 3]) -> f64 {
    (a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)
}

fn nearest(point: [f64; 3], centers: &[[f64; 3]]) -> usize {
    let mut selected = 0;
    let mut smallest = f64::INFINITY;
    for (index, center) in centers.iter().enumerate() {
        let error = distance2(point, *center);
        if error < smallest {
            smallest = error;
            selected = index;
        }
    }
    selected
}

/// Extract 1..=32 perceptual color clusters from a decoded RGBA8 buffer.
///
/// The caller must supply exactly width * height * 4 bytes, row-major,
/// unpremultiplied sRGB with four 8-bit RGBA channels per pixel. Color order
/// is by descending pixel population; ties follow deterministic center order.
/// The algorithm is weighted k-means in Oklab on 5-bit/channel quantized
/// histogram cells, seeded by a deterministic population-weighted farthest
/// point heuristic. It is **not** the ColorThief algorithm; the palette need
/// not match a legacy image-extraction tool byte-for-byte.
///
/// Memory usage depends on at most 32768 histogram cells, rather than raw
/// image size. Iterate 12 times; no randomness or outside packages.
pub fn extract_image_palette(
    rgba: &[u8],
    width: usize,
    height: usize,
    options: ImagePaletteOptions,
) -> Result<ImagePalette, ImageError> {
    let bytes = width
        .checked_mul(height)
        .and_then(|count| count.checked_mul(4))
        .ok_or(ImageError::InvalidDimensions)?;
    if width == 0 || height == 0 || rgba.len() != bytes {
        return Err(ImageError::InvalidBuffer);
    }
    if options.count == 0 || options.count > MAX_IMAGE_PALETTE_COLORS {
        return Err(ImageError::InvalidCount);
    }
    if options.stride == 0 || options.max_samples == 0 || options.max_samples > MAX_IMAGE_SAMPLES {
        return Err(ImageError::InvalidSampling);
    }
    let region = options.region.unwrap_or(ImageRegion {
        x: 0,
        y: 0,
        width,
        height,
    });
    if region.width == 0
        || region.height == 0
        || region.x.checked_add(region.width).is_none_or(|v| v > width)
        || region
            .y
            .checked_add(region.height)
            .is_none_or(|v| v > height)
    {
        return Err(ImageError::InvalidRegion);
    }

    // Adaptive 2-D stride for huge regions. This typically stays under the
    // sample cap, while the explicit break below handles skinny rectangles.
    let area = region.width.saturating_mul(region.height);
    let adaptive = ((area as f64 / options.max_samples as f64).sqrt().ceil() as usize).max(1);
    let stride = options.stride.max(adaptive);
    let mut histogram = BTreeMap::<u16, Cell>::new();
    let mut sampled_pixels = 0;
    let mut eligible_pixels = 0;

    'rows: for y in (region.y..region.y + region.height).step_by(stride) {
        for x in (region.x..region.x + region.width).step_by(stride) {
            if sampled_pixels == options.max_samples {
                break 'rows;
            }
            sampled_pixels += 1;
            let offset = (y * width + x) * 4;
            let [r, g, b, a] = <[u8; 4]>::try_from(&rgba[offset..offset + 4])
                .map_err(|_| ImageError::InvalidBuffer)?;
            if a < options.alpha_threshold
                || (options.ignore_near_white && r >= 245 && g >= 245 && b >= 245)
            {
                continue;
            }
            eligible_pixels += 1;
            let key = ((u16::from(r >> 3)) << 10) | ((u16::from(g >> 3)) << 5) | u16::from(b >> 3);
            let cell = histogram.entry(key).or_default();
            cell.count += 1;
            cell.rgb_sums[0] += u64::from(r);
            cell.rgb_sums[1] += u64::from(g);
            cell.rgb_sums[2] += u64::from(b);
        }
    }

    if eligible_pixels == 0 {
        return Err(ImageError::EmptyImage);
    }
    let mut points = Vec::with_capacity(histogram.len());
    for cell in histogram.into_values() {
        let rgb = cell
            .rgb_sums
            .map(|sum| sum as f64 / (cell.count as f64 * 255.0));
        let c = Color::new(ColorSpace::Srgb, rgb, 1.0)?;
        let oklab = c.to(ColorSpace::Oklab)?.channels();
        points.push(Point {
            count: cell.count,
            rgb,
            oklab,
        });
    }

    let k = options.count.min(points.len());
    // Stable initial seed: the most populated bin. Points are sorted by the
    // BTreeMap key, ensuring deterministic index tie-breaking.
    let first = points
        .iter()
        .enumerate()
        .max_by(|(ia, a), (ib, b)| a.count.cmp(&b.count).then_with(|| ib.cmp(ia)))
        .map(|(i, _)| i)
        .ok_or(ImageError::EmptyImage)?;
    let mut centers = vec![points[first].oklab];
    while centers.len() < k {
        let mut best_index = None;
        let mut best_weighted = -1.0_f64;
        for (index, point) in points.iter().enumerate() {
            let min_distance = centers.iter().fold(f64::INFINITY, |best, center| {
                best.min(distance2(point.oklab, *center))
            });
            let score = min_distance * point.count as f64;
            if score > best_weighted {
                best_weighted = score;
                best_index = Some(index);
            }
        }
        // Identical perceptual colors would have zero distance, in which
        // case adding more centers gives no new information.
        if best_weighted <= 1e-20 {
            break;
        }
        centers.push(points[best_index.ok_or(ImageError::EmptyImage)?].oklab);
    }

    for _ in 0..12 {
        let mut sums = vec![[0.0; 3]; centers.len()];
        let mut weights = vec![0usize; centers.len()];
        for point in &points {
            let cluster = nearest(point.oklab, &centers);
            weights[cluster] += point.count;
            for i in 0..3 {
                sums[cluster][i] += point.oklab[i] * point.count as f64;
            }
        }
        for (index, weight) in weights.into_iter().enumerate() {
            if weight != 0 {
                for i in 0..3 {
                    centers[index][i] = sums[index][i] / weight as f64;
                }
            }
        }
    }

    let mut populations = vec![0usize; centers.len()];
    let mut rgb_sums = vec![[0.0; 3]; centers.len()];
    for point in &points {
        let cluster = nearest(point.oklab, &centers);
        populations[cluster] += point.count;
        for i in 0..3 {
            rgb_sums[cluster][i] += point.rgb[i] * point.count as f64;
        }
    }
    let mut colors = Vec::new();
    for (index, &population) in populations.iter().enumerate() {
        if population == 0 {
            continue;
        }
        let rgb = rgb_sums[index].map(|value| (value / population as f64).clamp(0.0, 1.0));
        colors.push(ExtractedColor {
            color: Color::new(ColorSpace::Srgb, rgb, 1.0)?,
            population,
            proportion: population as f64 / eligible_pixels as f64,
        });
    }
    colors.sort_by(|a, b| b.population.cmp(&a.population));
    Ok(ImagePalette {
        colors,
        sampled_pixels,
        eligible_pixels,
    })
}
