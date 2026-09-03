import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import RollingNumber from "../../../components/widget/RollingNumber";

afterEach(() => cleanup());

describe("RollingNumber", () => {
  it("keeps separators static and rolls changed digits from units upward", () => {
    const { container, rerender } = render(<RollingNumber value="129" />);

    rerender(<RollingNumber value="1,130" />);

    const changedDigits = Array.from(
      container.querySelectorAll('[data-rolling="true"]'),
    );
    expect(changedDigits).toHaveLength(2);
    expect(changedDigits.map((digit) => digit.getAttribute("data-position"))).toEqual([
      "1",
      "0",
    ]);
    expect(changedDigits.map((digit) => digit.getAttribute("data-from"))).toEqual([
      "2",
      "9",
    ]);
    expect(changedDigits.map((digit) => digit.getAttribute("data-to"))).toEqual([
      "3",
      "0",
    ]);
    expect(changedDigits[0].querySelector("[style]")?.getAttribute("style")).toContain(
      "--rolling-delay: 24ms",
    );
    expect(changedDigits[1].querySelector("[style]")?.getAttribute("style")).toContain(
      "--rolling-delay: 0ms",
    );
    expect(screen.getByTestId("rolling-number")).toHaveAttribute(
      "data-value",
      "1,130",
    );
  });

  it("settles on the latest digit after the roll finishes", () => {
    const { container, rerender } = render(<RollingNumber value="8" />);

    rerender(<RollingNumber value="9" />);

    const rollingTrack = container.querySelector('[data-rolling="true"] > span');
    expect(rollingTrack).not.toBeNull();
    fireEvent.animationEnd(rollingTrack!);

    expect(container.querySelector('[data-rolling="true"]')).toBeNull();
    expect(screen.getByTestId("rolling-number")).toHaveTextContent("9");
  });
});
