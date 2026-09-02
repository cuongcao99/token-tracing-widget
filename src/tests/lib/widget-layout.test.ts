import { describe, expect, it } from "vitest";
import {
  WIDGET_DEFAULT_WIDTH,
  WIDGET_MAX_HEIGHT,
  WIDGET_MAX_WIDTH,
  WIDGET_MIN_HEIGHT,
  WIDGET_MIN_WIDTH,
  widgetHeightForContent,
  widgetHeightForVisibleProviders,
} from "../../lib/widget-layout";

describe("widget layout", () => {
  it("keeps a breathable target height for each visible-provider count", () => {
    expect(widgetHeightForVisibleProviders(0)).toBe(192);
    expect(widgetHeightForVisibleProviders(1)).toBe(244);
    expect(widgetHeightForVisibleProviders(2)).toBe(316);
  });

  it("clamps invalid provider counts to a safe window height", () => {
    expect(widgetHeightForVisibleProviders(-1)).toBe(WIDGET_MIN_HEIGHT);
    expect(widgetHeightForVisibleProviders(99)).toBe(316);
    expect(widgetHeightForVisibleProviders(Number.NaN)).toBe(WIDGET_MIN_HEIGHT);
  });

  it("leaves a 17px breathing gap after measured content before clamping", () => {
    expect(widgetHeightForContent(1)).toBe(244);
    expect(widgetHeightForContent(1, 200)).toBe(244);
    expect(widgetHeightForContent(1, 400)).toBe(417);
    expect(widgetHeightForContent(1, 999)).toBe(WIDGET_MAX_HEIGHT);
  });

  it("exposes the logical resize bounds used by the native shell", () => {
    expect(WIDGET_DEFAULT_WIDTH).toBe(440);
    expect(WIDGET_MIN_WIDTH).toBe(360);
    expect(WIDGET_MAX_WIDTH).toBe(720);
    expect(WIDGET_MIN_HEIGHT).toBe(192);
    expect(WIDGET_MAX_HEIGHT).toBe(520);
  });
});
