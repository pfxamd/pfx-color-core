/**
 * Real browser execution of the compiled Rust WASM, optional picker,
 * Color Study and workspace. No UI deployment or app edits.
 * Playwright is test infrastructure only, never a core/runtime dependency.
 */
import assert from "node:assert/strict";
import { createServer } from "node:http";
import { readFile } from "node:fs/promises";
import { resolve, extname, sep } from "node:path";
import { chromium, firefox } from "playwright";

const root = resolve(".");
const server = createServer(async (request, response) => {
  const pathname = new URL(request.url ?? "/", "http://localhost").pathname;
  if (pathname === "/") {
    response.writeHead(200, { "Content-Type": "text/html; charset=utf-8" });
    response.end("<!doctype html><html><head><meta charset='utf-8'></head><body><main>PFx browser runtime test</main></body></html>");
    return;
  }
  const file = resolve(root, "." + decodeURIComponent(pathname));
  if (!file.startsWith(root + sep)) {
    response.writeHead(403).end();
    return;
  }
  try {
    const data = await readFile(file);
    const mime = {
      ".wasm": "application/wasm",
      ".mjs": "text/javascript; charset=utf-8",
    }[extname(file)] ?? "application/octet-stream";
    response.writeHead(200, { "Content-Type": mime });
    response.end(data);
  } catch {
    response.writeHead(404).end();
  }
});
await new Promise(resolve => server.listen(0, "127.0.0.1", resolve));
const url = "http://127.0.0.1:" + server.address().port;
try {
  for (const [name, browserType] of [["Chromium", chromium], ["Firefox", firefox]]) {
    const browser = await browserType.launch({ headless: true });
    try {
      for (const viewport of [
        { width: 1440, height: 900 },
        { width: 390, height: 844 },
      ]) {
        const errors = [];
        const page = await browser.newPage({ viewport });
        page.on("pageerror", error => errors.push(error.message));
        try {
          const response = await page.goto(url, { waitUntil: "load" });
          assert.equal(response.status(), 200);
          const result = await page.evaluate(async () => {
            const { createPfxColorCore } = await import("/bindings/javascript/pfx-color-core.mjs");
            const { createPfxColorTools } = await import("/bindings/javascript/pfx-color-tools.mjs");
            const { createPfxColorsWorkspace, pfxGradientToCss } =
              await import("/bindings/javascript/pfx-color-workspace.mjs");
            const binary = await fetch("/target/wasm32-unknown-unknown/release/pfx_color_ffi.wasm");
            if (!binary.ok) throw new Error("Missing compiled .wasm");
            const core = await createPfxColorCore(await binary.arrayBuffer());
            const picker = createPfxColorTools(core);
            const red = core.parseCss("#ff0000");
            const blue = core.parseCss("#0000ff");
            const mixed = core.interpolate(red, blue, 0.5, { space: "srgb" });
            const absent = core.parseCssMissing("hsl(none 70% 50% / none)");
            const solid = core.parseCssMissing("hsl(120 70% 50% / .8)");
            const missingMix = core.interpolateCssMissing(absent, solid, .5, { space: "hsl" });
            const missingCss = core.formatCssMissing(absent);
            const relative = core.parseCssMissing(
              "rgb(from red calc(r - 127.5) g b / calc(alpha - 0.25))");
            const computedByBrowser = document.createElement("div");
            computedByBrowser.style.color = "rgb(from red calc(r - 127.5) g b / calc(alpha - 0.25))";
            document.body.append(computedByBrowser);
            const browserColor = getComputedStyle(computedByBrowser).color;
            computedByBrowser.remove();
            const generated = core.anchoredPalette([red, blue, red], {
              count: 5, space: "srgb", gamut: "clip",
            });
            const custom = core.customHarmony(
              core.parseCss("oklch(60% .05 30)"),
              [0, 120, 240], { target: "oklch" });
            const study = picker.generateColorStudy("#336699", { randomSeed: 1773 });
            const workspace = createPfxColorsWorkspace(core, "#336699");
            workspace.generateTonalPalette({ count: 7 });
            workspace.generateHarmony("triadic");
            workspace.createGradient([
              { color: "#ff0000", position: 0 },
              { color: "#0000ff", position: 1 },
            ], { interpolationSpace: "srgb" });
            const css = pfxGradientToCss(workspace.getState().gradient);
            const green = workspace.setColorFromGradient(0.5);
            const imageData = new ImageData(2, 2);
            imageData.data.set([
              255, 0, 0, 255, 255, 0, 0, 255,
              0, 0, 255, 255, 255, 0, 0, 255,
            ]);
            const extracted = core.extractImagePalette(imageData.data, 2, 2, { count: 2 });
            workspace.undo();
            return {
              mixed: mixed.channels,
              missingMask: absent.missingMask,
              missingMix: missingMix.channels,
              missingAlpha: missingMix.alpha,
              missingCss,
              relative: relative.channels,
              relativeAlpha: relative.alpha,
              browserColor,
              paletteLength: generated.length,
              paletteCenter: generated[2].color.channels,
              customLength: custom.length,
              studyLength: study.colors.length,
              tonalPreserved: workspace.getState().palette.colors.length,
              gradientCss: css,
              selected: green.color.hex,
              afterUndo: workspace.getState().color.hex,
              pickerHex: picker.setColorChannel("#ff0000", "hsl", 0, 120).hex,
              extractedCount: extracted.colors.length,
              extractedDominant: extracted.colors[0].color.channels,
              extractedPopulation: extracted.colors[0].population,
            };
          });
          assert.deepEqual(result.mixed, [0.5, 0, 0.5]);
          assert.equal(result.missingMask, 9);
          assert.ok(result.missingCss.includes("none"));
          assert.ok(Math.abs(result.missingMix[0] - 120) < 1e-8);
          assert.ok(Math.abs(result.missingAlpha - 0.8) < 1e-8);
          assert.ok(Math.abs(result.relative[0] - .5) < 1e-8);
          assert.ok(Math.abs(result.relativeAlpha - .75) < 1e-8);
          assert.ok(result.browserColor.includes("127.5") || result.browserColor.includes("0.5"));
          assert.equal(result.paletteLength, 5);
          assert.deepEqual(result.paletteCenter, [0, 0, 1]);
          assert.equal(result.customLength, 3);
          assert.equal(result.studyLength, 10);
          assert.equal(result.tonalPreserved, 7);
          assert.ok(result.gradientCss.startsWith("linear-gradient("));
          assert.equal(result.selected, "#800080");
          assert.equal(result.afterUndo, "#336699");
          assert.equal(result.pickerHex, "#00ff00");
          assert.equal(result.extractedCount, 2);
          assert.deepEqual(result.extractedDominant, [1, 0, 0]);
          assert.equal(result.extractedPopulation, 3);

          // Render real CSS gradients to PNG, decode those pixels inside the
          // same real browser, then compare them to independently sampled Rust
          // WASM pixels. Test-only browser APIs; zero new core dependencies.
          const fixtures = [
            {
              name: "rectangular linear",
              css: "linear-gradient(45deg in srgb, #ff0000 0%, #0000ff 100%)",
              kind: "linear", angle: 45,
              stops: [0, 1], positions: [[40, 30], [120, 60], [200, 80]],
            },
            {
              name: "ellipse farthest-corner",
              css: "radial-gradient(ellipse farthest-corner at 70px 50px in srgb, #ff0000 0%, #0000ff 100%)",
              kind: "radial", centerX: 70, centerY: 50,
              radialShape: "ellipse", radialExtent: "farthest-corner",
              stops: [0, 1], positions: [[70, 50], [120, 60], [180, 100]],
            },
            {
              name: "explicit circle",
              css: "radial-gradient(circle 65px at 95px 60px in srgb, #ff0000 0%, #0000ff 100%)",
              kind: "radial", centerX: 95, centerY: 60,
              radialShape: "circle", radialExtent: "explicit",
              radiusX: 65, radiusY: 65,
              stops: [0, 1], positions: [[110, 60], [140, 70], [170, 80]],
            },
            {
              name: "conic offset",
              css: "conic-gradient(from 30deg at 80px 45px in srgb, #ff0000 0%, #0000ff 100%)",
              kind: "conic", angle: 30, centerX: 80, centerY: 45,
              stops: [0, 1], positions: [[120, 45], [80, 80], [45, 30]],
            },
            {
              name: "repeating linear",
              css: "repeating-linear-gradient(90deg in srgb, #ff0000 20%, #0000ff 40%)",
              kind: "linear", angle: 90, repeating: true,
              stops: [0.2, 0.4], positions: [[36, 45], [64, 45], [113, 55], [172, 60]],
            },
            {
              name: "degenerate repeating average",
              css: "repeating-linear-gradient(90deg in srgb, #ff0000 40%, #0000ff 40%)",
              kind: "linear", angle: 90, repeating: true,
              stops: [0.4, 0.4], positions: [[20, 40], [120, 60], [200, 80]],
            },
          ];
          for (const fixture of fixtures) {
            const { supported } = await page.evaluate(css => {
              const previous = document.querySelector("#pfx-css-pixel-fixture");
              previous?.remove();
              const div = document.createElement("div");
              div.id = "pfx-css-pixel-fixture";
              div.style.cssText = "box-sizing:content-box;width:240px;height:120px;"
                + "margin:0;padding:0;border:0;background-color:white;";
              div.style.backgroundImage = css;
              document.body.append(div);
              return { supported: getComputedStyle(div).backgroundImage !== "none" };
            }, fixture.css);
            assert.equal(supported, true, name + " does not support " + fixture.name);
            const png = await page.locator("#pfx-css-pixel-fixture").screenshot();
            const actual = await page.evaluate(async ({ png, positions }) => {
              const image = new Image();
              image.src = "data:image/png;base64," + png;
              await image.decode();
              const canvas = document.createElement("canvas");
              canvas.width = image.width;
              canvas.height = image.height;
              const ctx = canvas.getContext("2d", { willReadFrequently: true });
              ctx.drawImage(image, 0, 0);
              const rgba = ctx.getImageData(0, 0, canvas.width, canvas.height).data;
              return positions.map(([x, y]) => Array.from(
                rgba.slice((y * canvas.width + x) * 4, (y * canvas.width + x) * 4 + 4)
              ));
            }, { png: png.toString("base64"), positions: fixture.positions });
            const expected = await page.evaluate(async fixture => {
              const { createPfxColorCore } = await import("/bindings/javascript/pfx-color-core.mjs");
              const binary = await fetch("/target/wasm32-unknown-unknown/release/pfx_color_ffi.wasm");
              const core = await createPfxColorCore(await binary.arrayBuffer());
              const stops = fixture.stops.map((position, index) => ({
                position, color: core.parseCss(index === 0 ? "#ff0000" : "#0000ff"),
              }));
              const handle = core.createCssGradient(stops, {
                width: 240, height: 120, kind: fixture.kind,
                angle: fixture.angle, centerX: fixture.centerX, centerY: fixture.centerY,
                radialShape: fixture.radialShape, radialExtent: fixture.radialExtent,
                radiusX: fixture.radiusX, radiusY: fixture.radiusY,
                repeating: fixture.repeating, space: "srgb", target: "srgb", gamut: "clip",
              });
              try {
                return fixture.positions.map(([x, y]) => {
                  const color = handle.samplePixel(x + .5, y + .5);
                  return color.channels.map(channel => Math.round(channel * 255));
                });
              } finally { handle.dispose(); }
            }, fixture);
            if (fixture.name === "degenerate repeating average") {
              // CSS Images 3 mandates the *premultiplied average* for a
              // zero-length repeat. Some browser engines instead render the
              // last stop. Do not weaken the Rust spec test to match this.
              // Keep comparing/logging the browser result as a known
              // divergence until the engines converge with the specification.
              const mismatches = actual.flatMap((pixel, index) =>
                expected[index].map((value, channel) =>
                  Math.abs(value - pixel[channel]) > 5
                    ? { index, channel, rust: value, browser: pixel[channel] }
                    : null).filter(Boolean));
              if (mismatches.length) {
                console.log(name, viewport.width + "x" + viewport.height,
                  "KNOWN SPEC/BROWSER DIVERGENCE: zero-length CSS repeating average",
                  JSON.stringify(mismatches));
              }
            } else {
              actual.forEach((pixel, index) => {
                expected[index].forEach((value, channel) => {
                  assert.ok(Math.abs(value - pixel[channel]) <= 5,
                    name + " " + fixture.name + " " + fixture.positions[index]
                    + " channel " + channel + ": Rust " + value
                    + " vs browser " + pixel[channel]);
                });
              });
            }
          }
          assert.deepEqual(errors, [], "Browser runtime errors: " + errors.join("; "));
          console.log(name, viewport.width + "x" + viewport.height, "PASS");
        } finally {
          await page.close();
        }
      }
    } finally {
      await browser.close();
    }
  }
} finally {
  await new Promise(resolve => server.close(resolve));
}
console.log("PFx Rust WASM browser tests: PASS");
