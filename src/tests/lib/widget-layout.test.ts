import { describe, expect, it } from "vitest";
import {
  WIDGET_DEFAULT_WIDTH,
  WIDGET_MAX_HEIGHT,
  WIDGET_MAX_WIDTH,
  WIDGET_MIN_HEIGHT,
  WIDGET_MIN_WIDTH,
  widgetHeightForVisibleProviders,
} from "../../lib/widget-layout";

describe("widget layout", () => {
  it("keeps a breathable target height for each visible-provider count", () => {
    expect(widgetHeightForVisibleProviders(0)).toBe(176);
    expect(widgetHeightForVisibleProviders(1)).toBe(228);
    expect(widgetHeightForVisibleProviders(2)).toBe(300);
  });

  it("clamps invalid provider counts to a safe window height", () => {
    expect(widgetHeightForVisibleProviders(-1)).toBe(WIDGET_MIN_HEIGHT);
    expect(widgetHeightForVisibleProviders(99)).toBe(300);
    expect(widgetHeightForVisibleProviders(Number.NaN)).toBe(WIDGET_MIN_HEIGHT);
  });

  it("exposes the logical resize bounds used by the native shell", () => {
    expect(WIDGET_DEFAULT_WIDTH).toBe(440);
    expect(WIDGET_MIN_WIDTH).toBe(360);
    expect(WIDGET_MAX_WIDTH).toBe(720);
    expect(WIDGET_MIN_HEIGHT).toBe(176);
    expect(WIDGET_MAX_HEIGHT).toBe(520);
  });
});
