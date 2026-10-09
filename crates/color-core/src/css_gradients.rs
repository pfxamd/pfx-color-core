//! Pixel-coordinate CSS-like gradient geometry independent of legacy normalized
//! Gradient. CSS Images 3 geometry: https://www.w3.org/TR/css-images-3/#gradients
//! Explicit dimensions in CSS px, CSS stop fixup, optional repeating patterns.
//! Deliberately not a full CSS gradient grammar or raster compositor.
use crate::gamut::GamutMap;
use crate::gradients::{GradientSample, GradientStop};
use crate::interpolation::{interpolate, HueMethod};
use crate::palettes::{mapped_output, MAX_PALETTE_COLORS};
use crate::spaces::{ColorError, ColorSpace};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CssRadialShape {
    Circle,
    Ellipse,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CssRadialExtent {
    ClosestSide,
    FarthestSide,
    ClosestCorner,
    FarthestCorner,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CssRadialSize {
    Extent(CssRadialExtent),
    /// Explicit nonnegative CSS px radii; circles require equal radii.
    Explicit { radius_x: f64, radius_y: f64 },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CssGradientKind {
    Linear { angle_degrees: f64 },
    Radial {
        center_x: f64,
        center_y: f64,
        shape: CssRadialShape,
        size: CssRadialSize,
    },
    Conic {
        from_degrees: f64,
        center_x: f64,
        center_y: f64,
    },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CssGradientOptions {
    pub width: f64,
    pub height: f64,
    pub kind: CssGradientKind,
    pub repeating: bool,
    pub interpolation_space: ColorSpace,
    pub target_space: ColorSpace,
    pub hue_method: HueMethod,
    pub gamut_map: GamutMap,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CssGradient {
    stops: Vec<GradientStop>,
    options: CssGradientOptions,
}

impl CssGradient {
    /// CSS fixup preserves stop order; a decreasing position is raised to the
    /// preceding position, and repeated positions produce hard transitions.
    /// Stop percentages are represented by fractions and may fall outside 0..1.
    pub fn new(stops: &[GradientStop], options: CssGradientOptions) -> Result<Self, ColorError> {
        if !(2..=MAX_PALETTE_COLORS).contains(&stops.len()) {
            return Err(ColorError::InvalidCount);
        }
        if !options.width.is_finite() || !options.height.is_finite()
            || options.width <= 0.0 || options.height <= 0.0
        {
            return Err(ColorError::InvalidRange);
        }
        let valid_center = |x: f64| x.is_finite();
        match options.kind {
            CssGradientKind::Linear { angle_degrees } => {
                if !angle_degrees.is_finite() { return Err(ColorError::InvalidAngle); }
            }
            CssGradientKind::Conic { from_degrees, center_x, center_y } => {
                if !from_degrees.is_finite() { return Err(ColorError::InvalidAngle); }
                if !valid_center(center_x) || !valid_center(center_y) {
                    return Err(ColorError::InvalidPosition);
                }
            }
            CssGradientKind::Radial { center_x, center_y, shape, size } => {
                if !valid_center(center_x) || !valid_center(center_y) {
                    return Err(ColorError::InvalidPosition);
                }
                if let CssRadialSize::Explicit { radius_x, radius_y } = size {
                    if !radius_x.is_finite() || !radius_y.is_finite()
                        || radius_x <= 0.0 || radius_y <= 0.0
                        || (shape == CssRadialShape::Circle && radius_x != radius_y)
                    {
                        return Err(ColorError::InvalidRange);
                    }
                }
            }
        }
        let mut fixed = Vec::with_capacity(stops.len());
        let mut previous = f64::NEG_INFINITY;
        for stop in stops {
            if !stop.position.is_finite() { return Err(ColorError::InvalidPosition); }
            let position = stop.position.max(previous);
            fixed.push(GradientStop { position, color: stop.color });
            previous = position;
        }
        Ok(Self { stops: fixed, options })
    }

    pub fn stops(&self) -> &[GradientStop] { &self.stops }
    pub fn options(&self) -> CssGradientOptions { self.options }

    fn radii(&self, cx: f64, cy: f64, shape: CssRadialShape, size: CssRadialSize) -> (f64, f64) {
        if let CssRadialSize::Explicit { radius_x, radius_y } = size {
            return (radius_x, radius_y);
        }
        let size = match size {
            CssRadialSize::Extent(extent) => extent,
            CssRadialSize::Explicit { .. } => unreachable!(),
        };
        let dx_near = cx.abs().min((self.options.width - cx).abs());
        let dy_near = cy.abs().min((self.options.height - cy).abs());
        let dx_far = cx.abs().max((self.options.width - cx).abs());
        let dy_far = cy.abs().max((self.options.height - cy).abs());
        let (dx, dy) = match size {
            CssRadialExtent::ClosestSide | CssRadialExtent::ClosestCorner => (dx_near, dy_near),
            CssRadialExtent::FarthestSide | CssRadialExtent::FarthestCorner => (dx_far, dy_far),
        };
        let corner = matches!(size, CssRadialExtent::ClosestCorner | CssRadialExtent::FarthestCorner);
        if shape == CssRadialShape::Circle {
            let radius = if corner { dx.hypot(dy) } else if size == CssRadialExtent::ClosestSide {
                dx.min(dy)
            } else { dx.max(dy) };
            return (radius, radius);
        }
        if !corner { return (dx, dy); }
        // CSS ellipse corner radii scale the respective side radii by the
        // factor that puts the selected corner on the ellipse.
        if dx == 0.0 || dy == 0.0 { return (dx, dy); }
        let factor = std::f64::consts::SQRT_2;
        (dx * factor, dy * factor)
    }

    pub fn progress_at(&self, x: f64, y: f64) -> Result<f64, ColorError> {
        if !x.is_finite() || !y.is_finite() {
            return Err(ColorError::InvalidPosition);
        }
        let progress = match self.options.kind {
            CssGradientKind::Linear { angle_degrees } => {
                let theta = angle_degrees.rem_euclid(360.0).to_radians();
                let dx = theta.sin();
                let dy = -theta.cos();
                let length = self.options.width * dx.abs() + self.options.height * dy.abs();
                let projection = (x - self.options.width / 2.0) * dx
                    + (y - self.options.height / 2.0) * dy;
                0.5 + projection / length
            }
            CssGradientKind::Radial { center_x, center_y, shape, size } => {
                let (rx, ry) = self.radii(center_x, center_y, shape, size);
                let dx = (x - center_x).abs();
                let dy = (y - center_y).abs();
                if rx == 0.0 || ry == 0.0 {
                    if dx == 0.0 && dy == 0.0 { 0.0 } else { f64::INFINITY }
                } else {
                    (dx / rx).hypot(dy / ry)
                }
            }
            CssGradientKind::Conic { from_degrees, center_x, center_y } => {
                let dx = x - center_x;
                let dy = y - center_y;
                if dx == 0.0 && dy == 0.0 { 0.0 } else {
                    (dx.atan2(-dy).to_degrees() - from_degrees).rem_euclid(360.0) / 360.0
                }
            }
        };
        if progress.is_nan() { return Err(ColorError::NonFiniteResult); }
        Ok(progress)
    }

    fn emit(&self, index: usize, position: f64) -> Result<GradientSample, ColorError> {
        let (color, mapped) = mapped_output(
            self.stops[index].color, self.options.target_space, self.options.gamut_map,
        )?;
        Ok(GradientSample { position, color, mapped })
    }

    /// The logical, unbounded line coordinate; outside end stops extends solid
    /// colors unless repeating is enabled. Repeating uses the first-last span.
    pub fn sample_progress(&self, position: f64) -> Result<GradientSample, ColorError> {
        if !position.is_finite() { return Err(ColorError::InvalidPosition); }
        let first = self.stops[0].position;
        let last = self.stops[self.stops.len() - 1].position;
        let p = if self.options.repeating {
            let span = last - first;
            if span == 0.0 {
                // A zero-length repeat is degenerate: no periodic interval.
                // The last stop is used instead of dividing by zero.
                return self.emit(self.stops.len() - 1, position);
            }
            let folded = (position - first).rem_euclid(span) + first;
            if !folded.is_finite() { return Err(ColorError::NonFiniteResult); }
            folded
        } else { position };
        let right = self.stops.partition_point(|stop| stop.position <= p);
        if right == 0 { return self.emit(0, position); }
        if right == self.stops.len() { return self.emit(right - 1, position); }
        let left = self.stops[right - 1];
        let following = self.stops[right];
        let fraction = (p - left.position) / (following.position - left.position);
        let mixed = interpolate(
            left.color, following.color, fraction, self.options.interpolation_space,
            self.options.hue_method,
        )?;
        let (color, mapped) = mapped_output(mixed, self.options.target_space, self.options.gamut_map)?;
        Ok(GradientSample { position, color, mapped })
    }

    /// x/y are CSS pixels (not normalized coordinates); for raster work,
    /// sample at pixel centers: x + 0.5 and y + 0.5.
    pub fn sample_pixel(&self, x: f64, y: f64) -> Result<GradientSample, ColorError> {
        self.sample_progress(self.progress_at(x, y)?)
    }
}
