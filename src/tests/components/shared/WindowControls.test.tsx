import { fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import WindowGrip from "../../../components/shared/WindowGrip";
import WindowResizeHandles from "../../../components/shared/WindowResizeHandles";

const { startCurrentWindowResize } = vi.hoisted(() => ({
  startCurrentWindowResize: vi.fn().mockResolvedValue(undefined),
}));

vi.mock("../../../lib/window-actions", () => ({
  startCurrentWindowResize,
}));

beforeEach(() => {
  startCurrentWindowResize.mockClear();
});

describe("window controls", () => {
  it("renders a decorative six-dot drag grip", () => {
    render(<WindowGrip />);

    expect(screen.getByTestId("window-grip")).toHaveAttribute("aria-hidden", "true");
    expect(screen.getAllByTestId("window-grip-dot")).toHaveLength(6);
  });

  it("starts native resize in every edge and corner direction", () => {
    render(<WindowResizeHandles windowName="widget" />);

    const handles = [
      ["Resize widget from top edge", "North"],
      ["Resize widget from top-right corner", "NorthEast"],
      ["Resize widget from right edge", "East"],
      ["Resize widget from bottom-right corner", "SouthEast"],
      ["Resize widget from bottom edge", "South"],
      ["Resize widget from bottom-left corner", "SouthWest"],
      ["Resize widget from left edge", "West"],
      ["Resize widget from top-left corner", "NorthWest"],
    ] as const;

    for (const [label, direction] of handles) {
      fireEvent.mouseDown(screen.getByRole("button", { name: label }), {
        button: 0,
      });
      expect(startCurrentWindowResize).toHaveBeenLastCalledWith(direction);
    }

    expect(startCurrentWindowResize).toHaveBeenCalledTimes(handles.length);
  });

  it("does not bubble a resize gesture into a draggable parent", () => {
    const parentMouseDown = vi.fn();
    render(
      <div onMouseDown={parentMouseDown}>
        <WindowResizeHandles windowName="settings" />
      </div>,
    );

    fireEvent.mouseDown(screen.getByRole("button", { name: "Resize settings from top edge" }), {
      button: 0,
    });

    expect(parentMouseDown).not.toHaveBeenCalled();
  });
});
