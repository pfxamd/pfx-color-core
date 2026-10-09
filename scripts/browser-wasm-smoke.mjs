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
            workspace.undo();
            return {
              mixed: mixed.channels,
              paletteLength: generated.length,
              paletteCenter: generated[2].color.channels,
              customLength: custom.length,
              studyLength: study.colors.length,
              tonalPreserved: workspace.getState().palette.colors.length,
              gradientCss: css,
              selected: green.color.hex,
              afterUndo: workspace.getState().color.hex,
              pickerHex: picker.setColorChannel("#ff0000", "hsl", 0, 120).hex,
            };
          });
          assert.deepEqual(result.mixed, [0.5, 0, 0.5]);
          assert.equal(result.paletteLength, 5);
          assert.deepEqual(result.paletteCenter, [0, 0, 1]);
          assert.equal(result.customLength, 3);
          assert.equal(result.studyLength, 10);
          assert.equal(result.tonalPreserved, 7);
          assert.ok(result.gradientCss.startsWith("linear-gradient("));
          assert.equal(result.selected, "#800080");
          assert.equal(result.afterUndo, "#336699");
          assert.equal(result.pickerHex, "#00ff00");
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
