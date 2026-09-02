import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import UsageLimits, {
  formatLimitReset,
  limitColor,
} from "../../../components/widget/UsageLimits";

describe("UsageLimits", () => {
  afterEach(() => cleanup());

  it("interpolates one Claude-toned quota color continuously", () => {
    expect(limitColor(-10)).toBe("hsl(0 65% 50%)");
    expect(limitColor(0)).toBe("hsl(0 65% 50%)");
    expect(limitColor(25)).toBe("hsl(34 59% 51%)");
    expect(limitColor(50)).toBe("hsl(67 52% 52%)");
    expect(limitColor(75)).toBe("hsl(101 46% 53%)");
    expect(limitColor(100)).toBe("hsl(134 39% 54%)");
    expect(limitColor(110)).toBe("hsl(134 39% 54%)");
  });

  it("shows remaining capacity instead of provider usage consumed", () => {
    render(
      <UsageLimits
        limits={[{ windowMinutes: 300, usedPercent: 14, resetsAt: 1_788_367_052 }]}
      />,
    );

    expect(screen.getByText("86%")).toBeInTheDocument();
    expect(
      screen.getByRole("progressbar", { name: "5h limit: 86% remaining" }),
    ).toHaveAttribute("aria-valuenow", "86");
  });

  it("formats reset windows", () => {
    const now = Date.parse("2026-01-01T00:00:00Z");
    expect(
      formatLimitReset(Math.floor((now + 90 * 60 * 1000) / 1000), now),
    ).toBe("Resets in 1h 30m");
  });

  it("renders only supported limits", () => {
    render(
      <UsageLimits
        limits={[
          { windowMinutes: 300, usedPercent: 12, resetsAt: 1_788_367_052 },
          { windowMinutes: 60, usedPercent: 90, resetsAt: 1_788_367_052 },
        ]}
      />,
    );

    expect(screen.getByText("5h limit")).toBeInTheDocument();
    expect(screen.queryByText("60m limit")).not.toBeInTheDocument();
  });
});
