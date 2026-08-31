import { describe, expect, it } from "vitest";
import { isThemeId, themeOrder, themeRegistry } from "../../lib/theme";

describe("theme registry", () => {
  it("exposes Claude as the current registered theme", () => {
    expect(themeRegistry).toEqual([{ id: "claude", label: "Claude" }]);
    expect(themeOrder).toEqual(["claude"]);
    expect(isThemeId("claude")).toBe(true);
  });

  it("rejects unknown theme ids", () => {
    expect(isThemeId("private-theme")).toBe(false);
    expect(isThemeId(undefined)).toBe(false);
  });
});
