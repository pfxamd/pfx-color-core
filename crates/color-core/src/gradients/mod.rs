//! Multi-stop gradients with hard-stop semantics and normalized geometry.
//!
//! A platform-independent, color-math gradient. Spatial sampling assumes a
//! unit square, with (0,0) top-left and (1,1) bottom-right. Exporters for CSS,
//! GPU shaders or raster images belong in bindings, not in this core module.

use crate::gamut::GamutMap;
use crate::interpolation::{interpolate, HueMethod};
use crate::palettes::{mapped_output, MAX_PALETTE_COLORS};
use crate::spaces::{Color, ColorError, ColorSpace};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GradientKind {
    /// CSS-like angle: 0 degrees points upwards, 90 degrees rightwards.
    Linear { angle_degrees: f64 },
    /// Radial distance divided by distance to the farthest square corner.
    Radial { center_x: f64, center_y: f64 },
    /// 0 degrees points up, advancing clockwise, from the given start angle.
    Conic {
        from_degrees: f64,
        center_x: f64,
        center_y: f64,
    },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GradientOptions {
    pub kind: GradientKind,
    pub interpolation_space: ColorSpace,
    pub target_space: ColorSpace,
    pub hue_method: HueMethod,
    pub gamut_map: GamutMap,
}

impl Default for GradientOptions {
    fn default() -> Self {
        Self {
            kind: GradientKind::Linear {
                angle_degrees: 90.0,
            },
            interpolation_space: ColorSpace::Oklch,
            target_space: ColorSpace::Srgb,
            hue_method: HueMethod::Shorter,
            gamut_map: GamutMap::OklchChroma,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GradientStop {
    pub position: f64,
    pub color: Color,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GradientSample {
    pub position: f64,
    pub color: Color,
    pub mapped: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Gradient {
    stops: Vec<GradientStop>,
    options: GradientOptions,
}

fn in_unit_range(value: f64) -> bool {
    value.is_finite() && (0.0..=1.0).contains(&value)
}

impl Gradient {
    /// Create a gradient with 2..=256 stops. Stable sorting preserves input
    /// order among repeated positions, enabling hard transitions.
    pub fn new(stops: &[GradientStop], options: GradientOptions) -> Result<Self, ColorError> {
        if !(2..=MAX_PALETTE_COLORS).contains(&stops.len()) {
            return Err(ColorError::InvalidCount);
        }
        if stops.iter().any(|s| !in_unit_range(s.position)) {
            return Err(ColorError::InvalidPosition);
        }
        match options.kind {
            GradientKind::Linear { angle_degrees } => {
                if !angle_degrees.is_finite() {
                    return Err(ColorError::InvalidAngle);
                }
            }
            GradientKind::Radial { center_x, center_y } => {
                if !in_unit_range(center_x) || !in_unit_range(center_y) {
                    return Err(ColorError::InvalidPosition);
                }
            }
            GradientKind::Conic {
                from_degrees,
                center_x,
                center_y,
            } => {
                if !from_degrees.is_finite() {
                    return Err(ColorError::InvalidAngle);
                }
                if !in_unit_range(center_x) || !in_unit_range(center_y) {
                    return Err(ColorError::InvalidPosition);
                }
            }
        }
        let mut ordered = stops.to_vec();
        ordered.sort_by(|a, b| a.position.total_cmp(&b.position));
        Ok(Self {
            stops: ordered,
            options,
        })
    }

    #[must_use]
    pub fn stops(&self) -> &[GradientStop] {
        &self.stops
    }

    #[must_use]
    pub const fn options(&self) -> GradientOptions {
        self.options
    }

    fn emit(&self, source: Color, position: f64) -> Result<GradientSample, ColorError> {
        let (color, mapped) =
            mapped_output(source, self.options.target_space, self.options.gamut_map)?;
        Ok(GradientSample {
            position,
            color,
            mapped,
        })
    }

    /// Sample normalized gradient progress [0,1], independently of geometry.
    ///
    /// For identical stop positions the LAST stop wins at the exact boundary,
    /// while sampling immediately before it uses the preceding segment.
    /// Outside the first/last stop coordinates, their colors extend outward.
    pub fn sample(&self, position: f64) -> Result<GradientSample, ColorError> {
        if !in_unit_range(position) {
            return Err(ColorError::InvalidPosition);
        }
        let right = self.stops.partition_point(|stop| stop.position <= position);
        if right == 0 {
            return self.emit(self.stops[0].color, position);
        }
        if right == self.stops.len() {
            return self.emit(self.stops[right - 1].color, position);
        }
        let left = self.stops[right - 1];
        if left.position == position {
            return self.emit(left.color, position);
        }
        let next = self.stops[right];
        let local = (position - left.position) / (next.position - left.position);
        let mixed = interpolate(
            left.color,
            next.color,
            local,
            self.options.interpolation_space,
            self.options.hue_method,
        )?;
        self.emit(mixed, position)
    }

    /// Sample a point in the normalized, top-left-origin unit square.
    ///
    /// Radial and conic center points are well-defined here (progress 0).
    /// These explicit normalized models do not claim pixel-perfect CSS
    /// gradient geometry for arbitrary aspect ratios or box shapes.
    pub fn sample_xy(&self, x: f64, y: f64) -> Result<GradientSample, ColorError> {
        if !in_unit_range(x) || !in_unit_range(y) {
            return Err(ColorError::InvalidPosition);
        }
        let progress = match self.options.kind {
            GradientKind::Linear { angle_degrees } => {
                let theta = angle_degrees.rem_euclid(360.0).to_radians();
                let dx = theta.sin();
                let dy = -theta.cos();
                let half_span = 0.5 * (dx.abs() + dy.abs());
                let projected = (x - 0.5) * dx + (y - 0.5) * dy;
                ((projected + half_span) / (2.0 * half_span)).clamp(0.0, 1.0)
            }
            GradientKind::Radial { center_x, center_y } => {
                let distance = (x - center_x).hypot(y - center_y);
                let radius = [
                    center_x.hypot(center_y),
                    (1.0 - center_x).hypot(center_y),
                    center_x.hypot(1.0 - center_y),
                    (1.0 - center_x).hypot(1.0 - center_y),
                ]
                .into_iter()
                .fold(0.0_f64, f64::max);
                (distance / radius).clamp(0.0, 1.0)
            }
            GradientKind::Conic {
                from_degrees,
                center_x,
                center_y,
            } => {
                let dx = x - center_x;
                let dy = y - center_y;
                if dx == 0.0 && dy == 0.0 {
                    0.0
                } else {
                    let degrees = dx.atan2(-dy).to_degrees();
                    (degrees - from_degrees).rem_euclid(360.0) / 360.0
                }
            }
        };
        self.sample(progress)
    }
}
