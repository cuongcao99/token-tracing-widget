import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

const readStylesheet = (path) =>
  readFileSync(new URL(path, import.meta.url), "utf8");

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

const settingsCss = readStylesheet("../../styles/settings.css");
const widgetCss = readStylesheet("../../styles/widget.css");
const previewCss = readStylesheet("../../design-preview.css");

describe("panel surface CSS", () => {
  it("keeps the settings scrollbar at the shell edge", () => {
    const settingsBody = stylesheetBlock(settingsCss, ".settings-page__body");
    expect(settingsCss).toContain("--settings-inline-padding: 32px;");
    expect(settingsBody).toContain(
      "margin-right: calc(-1 * var(--settings-inline-padding));",
    );
    expect(settingsBody).toContain(
      "padding-right: var(--settings-inline-padding);",
    );
    expect(settingsBody).toContain("scrollbar-gutter: stable;");
    expect(settingsCss).toMatch(
      /\.settings-page__body::-webkit-scrollbar-button\s*\{[^}]*display:\s*none/,
    );

    const previewBody = stylesheetBlock(previewCss, ".claude-settings-body");
    expect(previewCss).toContain("--settings-inline-padding: 32px;");
    expect(previewBody).toContain(
      "margin-right: calc(-1 * var(--settings-inline-padding));",
    );
    expect(previewBody).toContain(
      "padding-right: var(--settings-inline-padding);",
    );
  });

  it("keeps settings elevated while the widget surface stays shadow-free", () => {
    expect(settingsCss).not.toContain("0 1px 2px");
    expect(settingsCss).toMatch(/box-shadow:[^;{}]*-\d+px/);
    expect(settingsCss).toMatch(/border:\s*0/);
    expect(settingsCss).toMatch(/outline:\s*none/);

    expect(stylesheetBlock(widgetCss, ".widget")).toContain(
      "box-shadow: none;",
    );
    expect(stylesheetBlock(widgetCss, ".widget--light")).toContain(
      "box-shadow: none;",
    );
    expect(widgetCss).toMatch(/border:\s*0/);
    expect(widgetCss).toMatch(/outline:\s*none/);

    expect(previewCss).not.toContain("0 1px 2px");
    expect(previewCss).toMatch(
      /\.overlay-preview--claude \.overlay-window\s*\{[^}]*box-shadow:\s*none;/,
    );
  });
});
