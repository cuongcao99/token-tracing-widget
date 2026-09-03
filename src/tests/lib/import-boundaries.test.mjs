import { readFileSync, readdirSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";

const testDirectory = dirname(fileURLToPath(import.meta.url));
const sourceRoot = resolve(testDirectory, "../../");

function sourceFiles(directory) {
  return readdirSync(directory, { withFileTypes: true }).flatMap((entry) => {
    const path = join(directory, entry.name);
    if (entry.isDirectory()) return sourceFiles(path);
    return /\.(ts|tsx)$/.test(entry.name) ? [path] : [];
  });
}

describe("desktop import boundary", () => {
  it("keeps contracts free of Tauri imports", () => {
    const contractSource = sourceFiles(join(sourceRoot, "lib", "contracts"))
      .map((path) => readFileSync(path, "utf8"))
      .join("\n");

    expect(contractSource).not.toContain("@tauri-apps/api");
  });

  it("keeps production Tauri imports under lib/desktop", () => {
    const productionSources = [
      ...sourceFiles(join(sourceRoot, "lib")),
      ...sourceFiles(join(sourceRoot, "components")),
      ...sourceFiles(join(sourceRoot, "hooks")),
    ];
    const nonDesktopSources = productionSources.filter(
      (path) => !path.includes(`${join("lib", "desktop")}`),
    );
    const source = nonDesktopSources
      .map((path) => readFileSync(path, "utf8"))
      .join("\n");

    expect(source).not.toMatch(/@tauri-apps\/api/);
    const desktopSource = sourceFiles(join(sourceRoot, "lib", "desktop"))
      .map((path) => readFileSync(path, "utf8"))
      .join("\n");
    expect(desktopSource).toContain("@tauri-apps/api");
  });
});
