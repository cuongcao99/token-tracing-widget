import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

const readSource = (path) => readFileSync(new URL(path, import.meta.url), "utf8");

const widgetModulePaths = [
  "../../styles/widget/surface.module.css",
  "../../styles/widget/provider.module.css",
  "../../styles/widget/limits.module.css",
  "../../styles/widget/metrics.module.css",
  "../../styles/widget/total.module.css",
  "../../styles/widget/loading.module.css",
  "../../styles/widget/rolling-number.module.css",
  "../../styles/shared/branding.module.css",
  "../../styles/shared/window-controls.module.css",
];

const settingsModulePaths = [
  "../../styles/settings/surface.module.css",
  "../../styles/settings/forms.module.css",
  "../../styles/settings/theme-picker.module.css",
];

describe("widget CSS module boundaries", () => {
  it("keeps every D1 module local and provider agnostic", () => {
    for (const path of widgetModulePaths) {
      const css = readSource(path);
      expect(css).not.toContain(":global");
      expect(css).not.toMatch(/var\(\s*--claude-/);
      expect(css).not.toMatch(/\.(?:[^{}\n]*)(?:claude|codex)/i);
    }
  });

  it("keeps theme declarations scoped to the root data attributes", () => {
    const themes = readSource("../../styles/globals/themes.css");

    expect(themes).toContain('[data-theme="claude"][data-color-mode="light"]');
    expect(themes).toContain('[data-theme="claude"][data-color-mode="dark"]');
    expect(themes).not.toContain(".theme--");
    expect(themes).not.toContain(".widget--");
  });

  it("loads a widget-only foundation and module graph from main", () => {
    const main = readSource("../../main.tsx");

    for (const path of [
      "./styles/globals/reset.css",
      "./styles/globals/tokens.css",
      "./styles/globals/themes.css",
      "./styles/widget/surface.module.css",
      "./styles/widget/provider.module.css",
      "./styles/widget/metrics.module.css",
      "./styles/widget/total.module.css",
      "./styles/widget/loading.module.css",
      "./styles/widget/rolling-number.module.css",
      "./styles/shared/branding.module.css",
      "./styles/shared/window-controls.module.css",
    ]) {
      expect(main).toContain(path);
    }

    expect(main).not.toContain("./styles/index.css");
    expect(main).not.toContain("styles/settings/");
  });

  it("keeps the settings entry out of the widget surface graph", () => {
    const settingsMain = readSource("../../settings-main.tsx");

    for (const path of [
      "./styles/globals/reset.css",
      "./styles/globals/tokens.css",
      "./styles/globals/themes.css",
      "./styles/settings/surface.module.css",
      "./styles/settings/forms.module.css",
      "./styles/settings/theme-picker.module.css",
      "./styles/shared/branding.module.css",
      "./styles/shared/window-controls.module.css",
    ]) {
      expect(settingsMain).toContain(path);
    }

    expect(settingsMain).not.toContain("./styles/index.css");
    expect(settingsMain).not.toContain("styles/widget/");
  });

  it("keeps every settings module local without provider selectors or fallbacks", () => {
    for (const path of settingsModulePaths) {
      const css = readSource(path);
      expect(css).not.toContain(":global");
      expect(css).not.toMatch(/var\(\s*--claude-/);
      expect(css).not.toMatch(/var\([^)]*,/);
      expect(css).not.toMatch(/\.(?:[^{}\n]*)(?:claude|codex)/i);
    }
  });

  it("uses semantic color slots throughout component modules", () => {
    const css = widgetModulePaths.map(readSource).join("\n");

    expect(css).toContain("var(--color-canvas)");
    expect(css).toContain("var(--color-ink)");
    expect(css).toContain("var(--color-line-soft)");
    expect(css).not.toMatch(/#[0-9a-f]{3,8}\b/i);
    expect(css).not.toMatch(/rgba?\(/i);
  });

  it("keeps provider name typography inherited and themeable", () => {
    const branding = readSource("../../styles/shared/branding.module.css");
    const provider = readSource("../../styles/widget/provider.module.css");
    const tokens = readSource("../../styles/globals/tokens.css");

    expect(branding).toContain("font-size: inherit;");
    expect(branding).toContain("font-size: var(--provider-name-display-size);");
    expect(branding).toContain("font-size: var(--provider-name-ui-size);");
    expect(branding).toContain("font-weight: var(--provider-name-display-weight);");
    expect(branding).toContain(
      "letter-spacing: var(--provider-name-display-tracking);",
    );
    expect(branding).not.toContain("--provider-name-size");
    expect(branding).not.toContain("font-weight: 600");
    expect(branding).not.toContain("letter-spacing: -0.15px");
    expect(provider).toContain("--provider-name-display-size");
    expect(provider).toContain("--provider-name-ui-size");
    expect(tokens).toContain("--provider-name-display-weight: 600;");
    expect(tokens).toContain("--provider-name-display-tracking: -0.15px;");
  });

  it("keeps shared branding on a generic gap token with a widget override", () => {
    const branding = readSource("../../styles/shared/branding.module.css");
    const provider = readSource("../../styles/widget/provider.module.css");
    const tokens = readSource("../../styles/globals/tokens.css");

    expect(branding).toContain("margin-right: var(--provider-mark-gap);");
    expect(branding).not.toContain("--widget-provider-mark-gap");
    expect(tokens).toContain("--provider-mark-gap: 10px;");
    expect(tokens).not.toContain("--widget-provider-mark-gap");
    expect(provider).toContain("--provider-mark-gap: 10px;");
  });
});
