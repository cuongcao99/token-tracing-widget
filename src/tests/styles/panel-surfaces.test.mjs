import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

const readStylesheet = (path) =>
  readFileSync(new URL(path, import.meta.url), "utf8").replaceAll(
    "\r\n",
    "\n",
  );

const stylesheetBlock = (css, selector) => {
  const escapedSelector = selector.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const match = css.match(
    new RegExp(`${escapedSelector}\\s*\\{([^}]*)\\}`),
  );

  if (!match) {
    throw new Error(`Missing CSS selector: ${selector}`);
  }

  return match[1];
};

const settingsSurfaceCss = readStylesheet(
  "../../styles/settings/surface.module.css",
);
const settingsTokensCss = readStylesheet("../../styles/globals/tokens.css");
const settingsFormsCss = readStylesheet("../../styles/settings/forms.module.css");
const themePickerCss = readStylesheet(
  "../../styles/settings/theme-picker.module.css",
);
const widgetSurfaceCss = readStylesheet("../../styles/widget/surface.module.css");
const widgetProviderCss = readStylesheet("../../styles/widget/provider.module.css");
const brandingCss = readStylesheet("../../styles/shared/branding.module.css");

describe("panel surface CSS", () => {
  it("keeps the settings scrollbar at the shell edge", () => {
    const settingsBody = stylesheetBlock(settingsSurfaceCss, ".body");
    expect(settingsTokensCss).toContain("--settings-inline-padding: 32px;");
    expect(settingsSurfaceCss).toMatch(
      /--settings-left-padding:\s*calc\(\s*var\(--settings-inline-padding\)\s*\+\s*var\(--settings-scrollbar-gutter\)\s*\);/,
    );
    expect(settingsBody).toContain(
      "margin-right: calc(-1 * var(--settings-inline-padding));",
    );
    expect(settingsBody).toContain(
      "padding-right: var(--settings-inline-padding);",
    );
    expect(settingsBody).toContain("scrollbar-gutter: stable;");
    expect(settingsSurfaceCss).toMatch(
      /\.body::-webkit-scrollbar-button\s*\{[^}]*display:\s*none/,
    );
  });

  it("keeps the widget scrollbar aligned with settings without reflow", () => {
    const widgetProviderList = stylesheetBlock(
      widgetSurfaceCss,
      ".providerList",
    );

    expect(widgetProviderList).toContain("overflow-y: auto;");
    expect(widgetProviderList).toContain(
      "margin-right: calc(-1 * var(--widget-padding-inline));",
    );
    expect(widgetProviderList).toContain(
      "padding-right: var(--widget-padding-inline);",
    );
    expect(widgetProviderList).toContain(
      "scrollbar-color: var(--color-line) transparent;",
    );
    expect(widgetProviderList).toContain("scrollbar-gutter: stable;");
    expect(widgetProviderList).toContain("scrollbar-width: thin;");

    expect(widgetSurfaceCss).toContain(
      ".providerList::-webkit-scrollbar {\n  width: var(--settings-scrollbar-gutter);\n}",
    );
    expect(widgetSurfaceCss).toMatch(
      /\.providerList::-webkit-scrollbar-button\s*\{[^}]*display:\s*none[^}]*width:\s*0[^}]*height:\s*0/s,
    );
    expect(widgetSurfaceCss).toContain(
      ".providerList::-webkit-scrollbar-thumb {\n  background: var(--color-line);\n  border: 2px solid transparent;\n  border-radius: var(--radius-pill);\n  background-clip: padding-box;\n}",
    );
  });

  it("keeps settings elevated while the widget surface stays shadow-free", () => {
    expect(settingsTokensCss).toContain("--elevation-widget: none;");
    expect(settingsTokensCss).toContain(
      "--elevation-settings-light: 0 18px 42px -16px rgba(20, 20, 19, 0.26),\n    0 36px 82px -26px rgba(20, 20, 19, 0.24);",
    );
    expect(settingsTokensCss).toContain(
      "--elevation-settings-dark: 0 18px 42px -16px rgba(0, 0, 0, 0.54),\n    0 36px 82px -26px rgba(0, 0, 0, 0.48);",
    );
    expect(settingsSurfaceCss).toContain("box-shadow: var(--elevation-settings);");
    for (const token of [
      "var(--color-canvas)",
      "var(--color-ink)",
      "var(--color-line)",
      "var(--color-line-soft)",
    ]) {
      expect(settingsSurfaceCss).toContain(token);
    }
    expect(settingsSurfaceCss).toMatch(/border:\s*0/);
    expect(settingsSurfaceCss).toMatch(/outline:\s*none/);

    expect(stylesheetBlock(widgetSurfaceCss, ".root")).toContain(
      "box-shadow: var(--elevation-widget);",
    );
    expect(widgetSurfaceCss).toMatch(/border:\s*0/);
    expect(widgetSurfaceCss).toMatch(/outline:\s*none/);
  });

  it("keeps the theme control and provider marks on the approved visual system", () => {
    expect(themePickerCss).toContain(".button");
    expect(themePickerCss).toContain(".menu");
    expect(themePickerCss).toContain(".option");
    expect(themePickerCss).toContain("font-family: var(--font-display);");
    expect(settingsFormsCss).toContain("font-size: var(--type-settings-meta);");
    expect(settingsSurfaceCss).toContain(
      "--provider-name-display-size: calc(var(--type-settings-meta) + 4px);",
    );
    expect(widgetProviderCss).toContain("--provider-name-display-size: calc(");
    expect(brandingCss).toContain("width: var(--provider-mark-size);");
    expect(brandingCss).toContain('data-logo-variant="warm-mark"');
    expect(brandingCss).toContain(".mark[data-logo-variant=\"monochrome-mark\"] .image");
  });
});
