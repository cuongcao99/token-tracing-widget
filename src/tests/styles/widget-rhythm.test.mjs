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
const metricsCss = readStylesheet("../../styles/widget/metrics.module.css");
const totalCss = readStylesheet("../../styles/widget/total.module.css");
const limitsCss = readStylesheet("../../styles/widget/limits.module.css");
const activityCss = readStylesheet("../../styles/widget/activity.module.css");
const sessionsCss = readStylesheet("../../styles/widget/sessions.module.css");
const tokenCss = readStylesheet("../../styles/globals/tokens.css");
const layoutSource = readStylesheet("../../lib/widget-layout.ts");

describe("widget vertical rhythm", () => {
  it("keeps vertical rhythm stable while the native window resizes", () => {
    expect(tokenCss).toContain("--widget-rhythm: 12px;");
    expect(tokenCss).not.toContain("cqh");
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

  it("uses matching UI numerals for quota and keeps token totals supportive", () => {
    expect(stylesheetBlock(limitsCss, ".value")).toContain(
      "font-family: var(--font-ui);",
    );
    expect(stylesheetBlock(limitsCss, ".value")).toContain(
      "font-size: calc(var(--type-widget-limit-value) - 2px);",
    );
    expect(stylesheetBlock(metricsCss, ".value")).toContain(
      "font-family: var(--font-ui);",
    );
    expect(stylesheetBlock(metricsCss, ".value")).toContain(
      "color: var(--color-muted);",
    );
    expect(stylesheetBlock(totalCss, ".value")).toContain(
      "color: var(--color-muted-soft);",
    );
    expect(tokenCss).toContain("--type-widget-limit-value: 16px;");
    expect(tokenCss).toContain("--type-widget-total: 16px;");
  });

  it("keeps waiting activity phrases neutral and empty state readable", () => {
    expect(activityCss).not.toContain('.phrase[data-state="idle"]');
    expect(activityCss).not.toContain('.phrase[data-state="stale"]');
    expect(stylesheetBlock(providerCss, ".emptyState")).toContain(
      "color: var(--color-muted);",
    );
    expect(stylesheetBlock(providerCss, ".emptyState")).not.toContain(
      "font-style: italic;",
    );
    expect(stylesheetBlock(providerCss, ".emptyState")).not.toContain(
      "opacity:",
    );
  });

  it("marks an active activity phrase with the coral accent", () => {
    expect(stylesheetBlock(activityCss, ".phrase")).toContain(
      "color: var(--color-muted);",
    );
    expect(stylesheetBlock(activityCss, '.phrase[data-state="active"]')).toContain(
      "color: var(--color-accent);",
    );
  });

  it("keeps active sessions distinct and long labels beside a fixed token column", () => {
    expect(stylesheetBlock(sessionsCss, ".labelGroup")).toContain("min-width: 0;");
    expect(stylesheetBlock(sessionsCss, ".currentLabel")).toContain(
      "color: var(--color-positive);",
    );
    expect(stylesheetBlock(sessionsCss, ".row")).toContain(
      "grid-template-columns: minmax(0, 1fr) max-content;",
    );
    expect(stylesheetBlock(sessionsCss, ".label")).toContain(
      "text-overflow: ellipsis;",
    );
  });

  it("uses the total's UI numerals for smaller session values", () => {
    expect(stylesheetBlock(sessionsCss, ".tokens")).toContain(
      "font-family: var(--font-ui);",
    );
    expect(stylesheetBlock(sessionsCss, ".tokens")).toContain(
      "font-size: var(--type-widget-meta);",
    );
    expect(stylesheetBlock(sessionsCss, ".tokens")).toContain(
      "color: var(--color-muted-soft);",
    );
  });

  it("keeps the widget scrollbar visible without introducing a brand color", () => {
    expect(tokenCss).toContain("--widget-scrollbar-width: 10px;");
    expect(stylesheetBlock(surfaceCss, ".providerList")).toContain(
      "scrollbar-color: var(--color-muted-soft) transparent;",
    );
    expect(stylesheetBlock(surfaceCss, ".providerList::-webkit-scrollbar")).toContain(
      "width: var(--widget-scrollbar-width);",
    );
    expect(stylesheetBlock(surfaceCss, ".providerList::-webkit-scrollbar-thumb")).toContain(
      "background: var(--color-muted-soft);",
    );
  });

  it("uses restrained motion with reduced-motion fallbacks", () => {
    expect(tokenCss).toContain(
      "--motion-easing: cubic-bezier(0.16, 1, 0.3, 1);",
    );
    expect(tokenCss).toContain("--motion-layout-duration: 150ms;");
    expect(surfaceCss).toContain("animation: widget-arrive");
    expect(surfaceCss).toContain("@media (prefers-reduced-motion: reduce)");
    expect(providerCss).toContain(
      "animation: provider-section-arrive var(--motion-layout-duration)",
    );
    expect(stylesheetBlock(limitsCss, ".fill")).toContain(
      "width 220ms var(--motion-easing)",
    );
    expect(sessionsCss).toContain(
      "animation: session-row-arrive var(--motion-layout-duration)",
    );
    expect(providerCss).toContain("@keyframes provider-section-arrive");
    expect(sessionsCss).toContain("@media (prefers-reduced-motion: reduce)");
  });

  it("gives updated timestamps room for descenders", () => {
    expect(stylesheetBlock(metricsCss, ".updated")).toContain(
      "line-height: 1.3;",
    );
  });
});
