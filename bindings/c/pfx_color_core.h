/* PFx Color Core C ABI, revision 1. Apache-2.0.
 * This header and the linked Rust library have no external runtime packages.
 * Compile/link with the resulting native pfx_color_ffi shared/static library.
 *
 * All PfxColor fields are numeric (double precision). RGB outside [0,1] is
 * deliberately permitted; gamut decisions are explicit.
 * PfxColor* pointers may point to caller-owned, initialized stack values.
 * Palette and gradient handles MUST be created/freed by the provided methods.
 * Do not free the same owned handle twice. The caller is responsible for
 * pointer validity and thread synchronization across the ABI boundary.
 *
 * Return codes: 0 success; -1 null pointer; -2 unknown enum; -3 invalid
 * color/value/operation; -4 invalid index. Scalar errors return NAN.
 */
#ifndef PFX_COLOR_CORE_H
#define PFX_COLOR_CORE_H

#include <stdint.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

enum PfxColorSpace {
    PFX_SRGB = 0,
    PFX_SRGB_LINEAR = 1,
    PFX_DISPLAY_P3 = 2,
    PFX_DISPLAY_P3_LINEAR = 3,
    PFX_REC2020 = 4,
    PFX_REC2020_LINEAR = 5,
    PFX_XYZ_D65 = 6,
    PFX_XYZ_D50 = 7,
    PFX_LAB = 8,
    PFX_LCH = 9,
    PFX_OKLAB = 10,
    PFX_OKLCH = 11,
    PFX_HSL = 12,
    PFX_HWB = 13,
    PFX_HSV = 14,
    PFX_A98_RGB = 15,
    PFX_PROPHOTO_RGB = 16,
    PFX_LAB_D65 = 17
};

enum PfxDifferenceMethod {
    PFX_DELTA_E_76 = 0,
    PFX_DELTA_E_2000 = 1,
    PFX_DELTA_E_OK = 2
};

enum PfxGamutMethod {
    PFX_GAMUT_CLIP = 0,
    PFX_GAMUT_OKLCH_CHROMA = 1,
    PFX_GAMUT_CSS = 2
};

enum PfxHueMethod {
    PFX_HUE_SHORTER = 0,
    PFX_HUE_LONGER = 1,
    PFX_HUE_INCREASING = 2,
    PFX_HUE_DECREASING = 3
};

enum PfxHarmonyScheme {
    PFX_HARMONY_ANALOGOUS = 0,
    PFX_HARMONY_COMPLEMENTARY = 1,
    PFX_HARMONY_SPLIT_COMPLEMENTARY = 2,
    PFX_HARMONY_TRIADIC = 3,
    PFX_HARMONY_TETRADIC = 4,
    PFX_HARMONY_SQUARE = 5
};

enum PfxGradientKind {
    PFX_GRADIENT_LINEAR = 0,
    PFX_GRADIENT_RADIAL = 1,
    PFX_GRADIENT_CONIC = 2
};

typedef struct PfxColor {
    uint32_t space;
    uint32_t reserved; /* always zero */
    double channels[3];
    double alpha;
} PfxColor;

#if defined(__cplusplus)
static_assert(sizeof(PfxColor) == 40, "PFx v1 color ABI size changed");
static_assert(offsetof(PfxColor, channels) == 8, "PFx v1 color ABI offsets changed");
#elif defined(__STDC_VERSION__) && __STDC_VERSION__ >= 201112L
_Static_assert(sizeof(PfxColor) == 40, "PFx v1 color ABI size changed");
_Static_assert(offsetof(PfxColor, channels) == 8, "PFx v1 color ABI offsets changed");
#endif

typedef struct PfxPalette PfxPalette;
typedef struct PfxAnchors PfxAnchors;
typedef struct PfxCustomHarmony PfxCustomHarmony;
typedef struct PfxGradient PfxGradient;
typedef struct PfxStudy PfxStudy;

uint32_t pfx_abi_version(void);
/* Byte buffers are owned by Rust; return with the EXACT original length. */
uint8_t *pfx_buffer_new(uint32_t length);
void pfx_buffer_free(uint8_t *ptr, uint32_t length);
int32_t pfx_css_parse(const uint8_t *utf8, uint32_t byte_length, PfxColor *out);
/* Returns string length excluding trailing NUL or negative error. */
int32_t pfx_css_format(const PfxColor *color, uint32_t format_kind,
                       uint32_t mapping, uint8_t *out, uint32_t capacity);
uint32_t pfx_color_size(void);
PfxColor *pfx_color_new(void);
void pfx_color_free(PfxColor *color);
int32_t pfx_color_set(
    PfxColor *color, uint32_t space, double c0, double c1, double c2, double alpha
);
uint32_t pfx_color_get_space(const PfxColor *color);
double pfx_color_get_channel(const PfxColor *color, uint32_t index);
double pfx_color_get_alpha(const PfxColor *color);

int32_t pfx_color_convert(const PfxColor *color, uint32_t target, PfxColor *out);
double pfx_color_difference(const PfxColor *a, const PfxColor *b, uint32_t method);
double pfx_color_contrast(const PfxColor *a, const PfxColor *b);
/* Signed APCA-W3 Lc, foreground first; not a WCAG contrast ratio. */
double pfx_color_apca(const PfxColor *foreground, const PfxColor *background);
double pfx_color_luminance(const PfxColor *color);
int32_t pfx_color_is_in_gamut(const PfxColor *color, uint32_t target);
int32_t pfx_color_map(
    const PfxColor *color, uint32_t target, uint32_t method, PfxColor *out
);
int32_t pfx_color_interpolate(
    const PfxColor *a, const PfxColor *b, double fraction,
    uint32_t interpolation_space, uint32_t hue_method, PfxColor *out
);

/* Owned opaque handles. Null indicates an invalid input/option. */
PfxPalette *pfx_palette_tonal_new(
    const PfxColor *seed, uint32_t count, double min_lightness,
    double max_lightness, double chroma_scale, uint32_t target, uint32_t gamut_method
);
PfxPalette *pfx_palette_ramp_new(
    const PfxColor *a, const PfxColor *b, uint32_t count,
    uint32_t interpolation_space, uint32_t target,
    uint32_t hue_method, uint32_t gamut_method
);
PfxPalette *pfx_palette_harmony_new(
    const PfxColor *seed, uint32_t scheme, double analogous_angle,
    double split_angle, double tetradic_angle, uint32_t target, uint32_t gamut_method
);
/* Owned 2..256-anchor palette builder; no input-array pointer arithmetic
   or copied foreign Rust-owned memory. Builders remain reusable after finish. */
PfxAnchors *pfx_anchors_new(void);
int32_t pfx_anchors_add(PfxAnchors *builder, const PfxColor *color);
PfxPalette *pfx_anchors_palette(
    const PfxAnchors *builder, uint32_t count,
    uint32_t interpolation_space, uint32_t target,
    uint32_t hue_method, uint32_t gamut_method
);
void pfx_anchors_free(PfxAnchors *builder);

/* Custom 2..256 finite hue offsets in degrees. */
PfxCustomHarmony *pfx_custom_harmony_new(
    const PfxColor *seed, uint32_t target, uint32_t gamut_method
);
int32_t pfx_custom_harmony_add(PfxCustomHarmony *builder, double hue_offset);
PfxPalette *pfx_custom_harmony_palette(const PfxCustomHarmony *builder);
void pfx_custom_harmony_free(PfxCustomHarmony *builder);

uint32_t pfx_palette_len(const PfxPalette *palette);
int32_t pfx_palette_get(const PfxPalette *palette, uint32_t index, PfxColor *out);
double pfx_palette_position(const PfxPalette *palette, uint32_t index);
int32_t pfx_palette_mapped(const PfxPalette *palette, uint32_t index);
double pfx_palette_hue_offset(const PfxPalette *palette, uint32_t index);
void pfx_palette_free(PfxPalette *palette);

/* Gradient handles: create -> add 2..256 stops -> sample -> free. */
PfxGradient *pfx_gradient_new(
    uint32_t kind, double angle_degrees, double center_x, double center_y,
    uint32_t interpolation_space, uint32_t target,
    uint32_t hue_method, uint32_t gamut_method
);
int32_t pfx_gradient_add_stop(
    PfxGradient *gradient, double position, const PfxColor *color
);
int32_t pfx_gradient_sample(
    const PfxGradient *gradient, double position, PfxColor *out
);
int32_t pfx_gradient_sample_xy(
    const PfxGradient *gradient, double x, double y, PfxColor *out
);
void pfx_gradient_free(PfxGradient *gradient);


/* Deterministic Color Study with exactly 10 swatches and seedable RNG. */
PfxStudy *pfx_study_new(
    const PfxColor *seed, uint32_t rnd_seed,
    double lightness, double chroma, double hue_range, double tone_range,
    uint32_t target_space, uint32_t gamut_method
);
uint32_t pfx_study_len(const PfxStudy *study);
uint32_t pfx_study_scheme(const PfxStudy *study);
int32_t pfx_study_get(const PfxStudy *study, uint32_t index, PfxColor *out);
int32_t pfx_study_mapped(const PfxStudy *study, uint32_t index);
double pfx_study_oklch(const PfxStudy *study, uint32_t index, uint32_t coordinate);
void pfx_study_free(PfxStudy *study);

#ifdef __cplusplus
}
#endif
#endif /* PFX_COLOR_CORE_H */
