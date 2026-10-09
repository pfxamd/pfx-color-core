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


    PfxAnchors *anchors = pfx_anchors_new();
    CHECK(anchors != NULL);
    CHECK(pfx_anchors_add(anchors, &black) == 0);
    CHECK(pfx_anchors_add(anchors, &white) == 0);
    CHECK(pfx_anchors_add(anchors, &black) == 0);
    PfxPalette *multi = pfx_anchors_palette(
        anchors, 5, PFX_SRGB, PFX_SRGB, PFX_HUE_SHORTER, PFX_GAMUT_CLIP
    );
    CHECK(multi != NULL);
    CHECK(pfx_palette_len(multi) == 5);
    CHECK(pfx_palette_get(multi, 2, &out) == 0);
    CHECK(fabs(out.channels[0] - 1.0) < 1e-12);
    pfx_palette_free(multi);
    pfx_anchors_free(anchors);

    PfxCustomHarmony *custom = pfx_custom_harmony_new(
        &green, PFX_SRGB, PFX_GAMUT_OKLCH_CHROMA
    );
    CHECK(custom != NULL);
    CHECK(pfx_custom_harmony_add(custom, -45.0) == 0);
    CHECK(pfx_custom_harmony_add(custom, 60.0) == 0);
    PfxPalette *custom_colors = pfx_custom_harmony_palette(custom);
    CHECK(custom_colors != NULL);
    CHECK(pfx_palette_len(custom_colors) == 2);
    CHECK(fabs(pfx_palette_hue_offset(custom_colors, 0) + 45.0) < 1e-12);
    CHECK(fabs(pfx_palette_hue_offset(custom_colors, 1) - 60.0) < 1e-12);
    pfx_palette_free(custom_colors);
    pfx_custom_harmony_free(custom);

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

    /* Real decoded RGBA8 native image extraction, no image dependency. */
    const uint8_t rgba[] = {
        255, 0, 0, 255, 255, 0, 0, 255,
        0, 0, 255, 255, 255, 0, 0, 255
    };
    PfxImagePalette *image = pfx_image_new(
        rgba, sizeof(rgba), 2, 2, 2, 1, 100, 128, 0, 0, 0, 0, 0
    );
    CHECK(image != NULL);
    CHECK(pfx_image_len(image) == 2);
    CHECK(pfx_image_sampled(image) == 4);
    CHECK(pfx_image_eligible(image) == 4);
    CHECK(pfx_image_population(image, 0) == 3);
    CHECK(fabs(pfx_image_proportion(image, 0) - 0.75) < 1e-12);
    CHECK(pfx_image_get(image, 0, &out) == 0);
    CHECK(out.space == PFX_SRGB && fabs(out.channels[0] - 1.0) < 1e-12);
    CHECK(pfx_image_get(image, 3, &out) == -4);
    pfx_image_free(image);
    uint8_t *upload = pfx_image_buffer_new(sizeof(rgba));
    CHECK(upload != NULL);
    pfx_image_buffer_free(upload, sizeof(rgba));
    CHECK(pfx_image_new(NULL, sizeof(rgba), 2, 2, 2, 1, 100, 128, 0,
                        0, 0, 0, 0) == NULL);


    /* Separate 48-byte CSS missing channel wire interface; v1 numeric ABI stays unchanged. */
    CHECK(pfx_css_missing_color_size() == 48);
    PfxCssColor *css_a = pfx_css_missing_color_new();
    PfxCssColor *css_b = pfx_css_missing_color_new();
    PfxCssColor *css_out = pfx_css_missing_color_new();
    CHECK(css_a != NULL && css_b != NULL && css_out != NULL);
    const char source_none[] = "oklch(none 0.2 none / none)";
    const char source_color[] = "oklch(0.6 0.3 250 / 0.4)";
    CHECK(pfx_css_missing_color_parse((const uint8_t*)source_none,
          sizeof(source_none) - 1, css_a) == 0);
    CHECK(pfx_css_missing_color_get_mask(css_a) == 13);
    CHECK(pfx_css_missing_color_parse((const uint8_t*)source_color,
          sizeof(source_color) - 1, css_b) == 0);
    CHECK(pfx_css_missing_color_interpolate(css_a, css_b, 0.5,
          PFX_OKLCH, PFX_HUE_SHORTER, css_out) == 0);
    CHECK(pfx_css_missing_color_get_mask(css_out) == 0);
    CHECK(fabs(pfx_css_missing_color_get_channel(css_out, 2) - 250) < 1e-8);
    char missingbuf[256] = {0};
    CHECK(pfx_css_missing_color_format(css_a, (uint8_t*)missingbuf,
          sizeof(missingbuf)) > 0);
    CHECK(pfx_css_missing_color_convert(css_a, PFX_SRGB, css_out) == 0);
    CHECK(pfx_css_missing_color_get_mask(css_out) == 0);
    CHECK(pfx_css_missing_color_set(css_out, PFX_SRGB, 1, 2, 3, 1, 16) < 0);
    pfx_css_missing_color_free(css_a);
    pfx_css_missing_color_free(css_b);
    pfx_css_missing_color_free(css_out);


    /* Pixel-dimension CSS gradient ABI, independent of legacy normalized handle. */
    PfxCssGradient *css_pixels = pfx_css_gradient_new(
        PFX_GRADIENT_LINEAR, 240.0, 120.0,
        45.0, 120.0, 60.0, 1, 3, 0.0, 0.0, 0,
        PFX_SRGB, PFX_SRGB, PFX_HUE_SHORTER, PFX_GAMUT_CLIP
    );
    CHECK(css_pixels != NULL);
    CHECK(pfx_css_gradient_add_stop(css_pixels, 0.0, &black) == 0);
    CHECK(pfx_css_gradient_add_stop(css_pixels, 1.0, &white) == 0);
    CHECK(pfx_css_gradient_sample_pixel(css_pixels, 0.0, 0.0, &out) == 0);
    CHECK(fabs(out.channels[0]) < 1e-12);
    CHECK(pfx_css_gradient_sample_pixel(css_pixels, 0.0, 120.0, &out) == 0);
    CHECK(fabs(out.channels[0] - 1.0 / 3.0) < 1e-10);
    CHECK(pfx_css_gradient_sample_progress(css_pixels, 0.5, &out) == 0);
    CHECK(fabs(out.channels[0] - 0.5) < 1e-12);
    CHECK(pfx_css_gradient_add_stop(css_pixels, NAN, &black) < 0);
    CHECK(pfx_css_gradient_sample_pixel(css_pixels, NAN, 50, &out) < 0);
    pfx_css_gradient_free(css_pixels);
    CHECK(pfx_css_gradient_new(
        PFX_GRADIENT_RADIAL, 0.0, 100.0, 0.0, 50.0, 50.0,
        1, 3, 0.0, 0.0, 0, PFX_SRGB, PFX_SRGB,
        PFX_HUE_SHORTER, PFX_GAMUT_CLIP
    ) == NULL);
    PfxCssGradient *css_radial = pfx_css_gradient_new(
        PFX_GRADIENT_RADIAL, 240.0, 120.0,
        0.0, 120.0, 60.0, 1, 3, 0.0, 0.0, 0,
        PFX_SRGB, PFX_SRGB, PFX_HUE_SHORTER, PFX_GAMUT_CLIP
    );
    CHECK(css_radial != NULL);
    CHECK(pfx_css_gradient_add_stop(css_radial, 0.0, &black) == 0);
    CHECK(pfx_css_gradient_add_stop(css_radial, 1.0, &white) == 0);
    CHECK(pfx_css_gradient_sample_pixel(css_radial, 240, 120, &out) == 0);
    CHECK(fabs(out.channels[0] - 1) < 1e-9);
    pfx_css_gradient_free(css_radial);

    CHECK(pfx_color_convert(NULL, PFX_SRGB, &out) == -1);
    CHECK(pfx_color_set(&out, 888, 0.0, 0.0, 0.0, 1.0) == -2);
    CHECK(isnan(pfx_color_difference(NULL, &white, PFX_DELTA_E_OK)));
    puts("PFx native C ABI smoke: PASS");
    return 0;
}
