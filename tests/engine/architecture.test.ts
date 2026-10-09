import { describe, expect, it } from "vitest";
import { readFile, readdir } from "node:fs/promises";
import { extname, join, relative } from "node:path";

async function files(directory: string): Promise<string[]> {
  const entries = await readdir(directory, { withFileTypes: true });
  const result: string[] = [];
  for (const entry of entries) {
    const path = join(directory, entry.name);
    if (entry.isDirectory()) result.push(...(await files(path)));
    else if ([".ts", ".tsx"].includes(extname(entry.name))) result.push(path);
  }
  return result;
}

describe("architecture boundaries", () => {
  it("keeps vendor imports inside engine adapters", async () => {
    const root = join(process.cwd(), "src");
    const violations: string[] = [];

    for (const file of await files(root)) {
      const source = await readFile(file, "utf8");
      const hasVendor =
        source.includes('from "colorjs.io"') ||
        source.includes('from "colorthief"');
      if (hasVendor && !relative(root, file).startsWith("engine/adapters/")) {
        violations.push(relative(root, file));
      }
    }

    expect(violations).toEqual([]);
  });
});
