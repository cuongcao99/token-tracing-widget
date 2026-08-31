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

const widgetCss = readStylesheet("../../styles/widget.css");
const previewCss = readStylesheet("../../design-preview.css");
const layoutSource = readStylesheet("../../lib/widget-layout.ts");

describe("widget vertical rhythm", () => {
  it("uses one bounded container-relative spacing token", () => {
    expect(widgetCss).toContain("container-type: size;");
    expect(widgetCss).toContain(
      "--widget-rhythm: clamp(10px, 4cqh, 20px);",
    );
    expect(layoutSource).toContain("WIDGET_MIN_HEIGHT = 192");
    expect(layoutSource).toContain("WIDGET_MAX_HEIGHT = 520");
  });

  it("uses the same rhythm after every horizontal boundary", () => {
    expect(stylesheetBlock(widgetCss, ".widget-header")).toContain(
      "padding-bottom: var(--widget-rhythm);",
    );
    expect(stylesheetBlock(widgetCss, ".widget-provider-list")).toContain(
      "padding-top: var(--widget-rhythm);",
    );
    expect(stylesheetBlock(widgetCss, ".widget-provider")).toContain(
      "padding: 0 0 var(--widget-rhythm);",
    );
    expect(
      stylesheetBlock(widgetCss, ".widget-provider + .widget-provider"),
    ).toContain("padding-top: var(--widget-rhythm);");
    expect(stylesheetBlock(widgetCss, ".widget-total")).toContain(
      "padding-top: var(--widget-rhythm);",
    );
  });

  it("keeps the static Claude preview on the same rhythm contract", () => {
    expect(previewCss).toContain("container-type: size;");
    expect(previewCss).toContain(
      "--widget-rhythm: clamp(10px, 4cqh, 20px);",
    );
    expect(previewCss).toContain(
      ".overlay-preview--claude .overlay-provider-list",
    );
    expect(previewCss).toContain(
      ".overlay-preview--claude .overlay-window__total",
    );
  });
});
