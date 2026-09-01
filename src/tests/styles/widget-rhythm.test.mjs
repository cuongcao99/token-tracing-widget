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

const surfaceCss = readStylesheet("../../styles/widget/surface.module.css");
const providerCss = readStylesheet("../../styles/widget/provider.module.css");
const totalCss = readStylesheet("../../styles/widget/total.module.css");
const tokenCss = readStylesheet("../../styles/globals/tokens.css");
const layoutSource = readStylesheet("../../lib/widget-layout.ts");

describe("widget vertical rhythm", () => {
  it("uses one bounded container-relative spacing token", () => {
    expect(surfaceCss).toContain("container-type: size;");
    expect(tokenCss).toContain(
      "--widget-rhythm: clamp(10px, 4cqh, 20px);",
    );
    expect(layoutSource).toContain("WIDGET_MIN_HEIGHT = 192");
    expect(layoutSource).toContain("WIDGET_MAX_HEIGHT = 520");
  });

  it("uses the same rhythm after every horizontal boundary", () => {
    expect(stylesheetBlock(surfaceCss, ".header")).toContain(
      "padding-bottom: var(--widget-rhythm);",
    );
    expect(stylesheetBlock(surfaceCss, ".providerList")).toContain(
      "padding-top: var(--widget-rhythm);",
    );
    expect(stylesheetBlock(providerCss, ".section")).toContain(
      "padding: 0 0 var(--widget-rhythm);",
    );
    expect(stylesheetBlock(providerCss, ".section + .section")).toContain(
      "padding-top: var(--widget-rhythm);",
    );
    expect(stylesheetBlock(totalCss, ".root")).toContain(
      "padding-top: var(--widget-rhythm);",
    );
  });
});
