/* Zero-dependency native C integration smoke test for PFx Color Core. */
#include "pfx_color_core.h"
#include <math.h>
#include <stdio.h>

#define CHECK(condition) \
    do { if (!(condition)) { \
        fprintf(stderr, "C ABI smoke failed at line %d: %s\n", __LINE__, #condition); \
        return 1; \
    } } while (0)

int main(void) {
    CHECK(pfx_abi_version() == 1);
    CHECK(pfx_color_size() == 40);
    PfxColor black = { PFX_SRGB, 0, {0.0, 0.0, 0.0}, 1.0 };
    PfxColor white = { PFX_SRGB, 0, {1.0, 1.0, 1.0}, 1.0 };
    PfxColor green = { PFX_DISPLAY_P3, 0, {0.0, 1.0, 0.0}, 1.0 };
    PfxColor out = {0};
    const char css[] = "hsl(120 100% 50% / 50%)";
    char hexbuf[64] = {0};
    CHECK(pfx_css_parse((const uint8_t*)css, (uint32_t)(sizeof(css) - 1), &out) == 0);
    CHECK(out.space == PFX_HSL);
    CHECK(fabs(out.alpha - 0.5) < 1e-12);
    CHECK(pfx_css_format(&out, 1, PFX_GAMUT_CLIP, (uint8_t*)hexbuf, sizeof(hexbuf)) == 9);
    CHECK(hexbuf[0] == '#');
    CHECK(pfx_color_convert(&out, PFX_SRGB, &out) == 0);
    CHECK(fabs(out.channels[1] - 1.0) < 1e-10);


    CHECK(fabs(pfx_color_contrast(&black, &white) - 21.0) < 1e-12);
    CHECK(fabs(pfx_color_difference(&black, &white, PFX_DELTA_E_OK) - 1.0) < 1e-7);
    CHECK(pfx_color_convert(&white, PFX_OKLAB, &out) == 0);
    CHECK(out.space == PFX_OKLAB);
    CHECK(fabs(out.channels[0] - 1.0) < 1e-7);
    CHECK(pfx_color_convert(&out, PFX_SRGB, &out) == 0); /* in-place */
    CHECK(fabs(out.channels[0] - 1.0) < 1e-7);

    CHECK(pfx_color_is_in_gamut(&green, PFX_SRGB) == 0);
    CHECK(pfx_color_map(&green, PFX_SRGB, PFX_GAMUT_OKLCH_CHROMA, &out) == 0);
    CHECK(pfx_color_is_in_gamut(&out, PFX_SRGB) == 1);

    CHECK(pfx_color_interpolate(&black, &white, 0.5, PFX_SRGB, PFX_HUE_SHORTER, &out) == 0);
    CHECK(fabs(out.channels[0] - 0.5) < 1e-12);

    PfxColor *owned = pfx_color_new();
    CHECK(owned != NULL);
    CHECK(pfx_color_set(owned, PFX_SRGB, 0.1, 0.2, 0.3, 0.4) == 0);
    CHECK(fabs(pfx_color_get_channel(owned, 1) - 0.2) < 1e-12);
    CHECK(fabs(pfx_color_get_alpha(owned) - 0.4) < 1e-12);
    pfx_color_free(owned);

    PfxPalette *tonal = pfx_palette_tonal_new(&green, 7, 0.12, 0.96, 1.0,
                                               PFX_SRGB, PFX_GAMUT_OKLCH_CHROMA);
    CHECK(tonal != NULL);
    CHECK(pfx_palette_len(tonal) == 7);
    CHECK(pfx_palette_get(tonal, 3, &out) == 0);
    CHECK(out.space == PFX_SRGB);
    CHECK(fabs(pfx_palette_position(tonal, 3) - 0.5) < 1e-12);
    pfx_palette_free(tonal);

    PfxPalette *ramp = pfx_palette_ramp_new(
        &black, &white, 3, PFX_SRGB, PFX_SRGB,
        PFX_HUE_SHORTER, PFX_GAMUT_CLIP
    );
    CHECK(ramp != NULL);
    CHECK(pfx_palette_get(ramp, 1, &out) == 0);
    CHECK(fabs(out.channels[0] - 0.5) < 1e-12);
    pfx_palette_free(ramp);

    PfxPalette *harmony = pfx_palette_harmony_new(
        &green, PFX_HARMONY_TRIADIC, 30.0, 30.0, 60.0,
        PFX_SRGB, PFX_GAMUT_OKLCH_CHROMA
    );
    CHECK(harmony != NULL);
    CHECK(pfx_palette_len(harmony) == 3);
    CHECK(fabs(pfx_palette_hue_offset(harmony, 1) - 120.0) < 1e-12);
    pfx_palette_free(harmony);

    PfxGradient *gradient = pfx_gradient_new(
        PFX_GRADIENT_LINEAR, 90.0, 0.5, 0.5,
        PFX_SRGB, PFX_SRGB, PFX_HUE_SHORTER, PFX_GAMUT_CLIP
    );
    CHECK(gradient != NULL);
    CHECK(pfx_gradient_add_stop(gradient, 0.0, &black) == 0);
    CHECK(pfx_gradient_add_stop(gradient, 1.0, &white) == 0);
    CHECK(pfx_gradient_sample(gradient, 0.5, &out) == 0);
    CHECK(fabs(out.channels[0] - 0.5) < 1e-12);
    CHECK(pfx_gradient_sample_xy(gradient, 1.0, 0.5, &out) == 0);
    CHECK(fabs(out.channels[0] - 1.0) < 1e-12);
    pfx_gradient_free(gradient);

    PfxStudy *study = pfx_study_new(&green, 321, 58.0, 58.0, 58.0, 58.0,
                                  PFX_SRGB, PFX_GAMUT_CSS);
    CHECK(study != NULL);
    CHECK(pfx_study_len(study) == 10);
    CHECK(pfx_study_scheme(study) < 6);
    CHECK(pfx_study_get(study, 9, &out) == 0);
    CHECK(out.space == PFX_SRGB);
    CHECK(isfinite(pfx_study_oklch(study, 4, 0)));
    pfx_study_free(study);

    CHECK(pfx_color_convert(NULL, PFX_SRGB, &out) == -1);
    CHECK(pfx_color_set(&out, 888, 0.0, 0.0, 0.0, 1.0) == -2);
    CHECK(isnan(pfx_color_difference(NULL, &white, PFX_DELTA_E_OK)));
    puts("PFx native C ABI smoke: PASS");
    return 0;
}
