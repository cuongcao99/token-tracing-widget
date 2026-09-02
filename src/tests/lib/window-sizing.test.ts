import { beforeEach, describe, expect, it, vi } from "vitest";
import { syncWidgetWindowHeight } from "../../lib/window-sizing";

const {
  getCurrentWindow,
  innerSize,
  scaleFactor,
  setSize,
  setSizeConstraints,
} = vi.hoisted(() => {
  const innerSize = vi.fn();
  const scaleFactor = vi.fn();
  const setSize = vi.fn();
  const setSizeConstraints = vi.fn();
  const getCurrentWindow = vi.fn(() => ({
    innerSize,
    scaleFactor,
    setSize,
    setSizeConstraints,
  }));
  return { getCurrentWindow, innerSize, scaleFactor, setSize, setSizeConstraints };
});

vi.mock("@tauri-apps/api/window", async () => {
  const actual = await vi.importActual<typeof import("@tauri-apps/api/window")>(
    "@tauri-apps/api/window",
  );
  return { ...actual, getCurrentWindow };
});

beforeEach(() => {
  innerSize.mockReset();
  scaleFactor.mockReset();
  setSize.mockReset();
  setSizeConstraints.mockReset();
  innerSize.mockResolvedValue({ width: 1200, height: 900 });
  scaleFactor.mockResolvedValue(2);
  setSize.mockResolvedValue(undefined);
  setSizeConstraints.mockResolvedValue(undefined);
});

describe("widget window sizing", () => {
  it("preserves the current logical width while changing only the responsive height", async () => {
    await syncWidgetWindowHeight(1);

    expect(setSize).toHaveBeenCalledTimes(1);
    expect(setSize.mock.calls[0][0]).toMatchObject({ width: 600, height: 244 });
    expect(setSizeConstraints).toHaveBeenCalledWith({
      minWidth: 360,
      minHeight: 244,
      maxWidth: 720,
      maxHeight: 520,
    });
    expect(setSizeConstraints.mock.invocationCallOrder[0]).toBeLessThan(
      setSize.mock.invocationCallOrder[0],
    );
  });

  it("clamps a manually resized width and lets the newest visibility state win", async () => {
    innerSize.mockResolvedValue({ width: 1800, height: 900 });

    await Promise.all([syncWidgetWindowHeight(1), syncWidgetWindowHeight(2)]);

    expect(setSize).toHaveBeenCalledTimes(1);
    expect(setSize.mock.calls[0][0]).toMatchObject({ width: 720, height: 316 });
    expect(setSizeConstraints).toHaveBeenCalledWith({
      minWidth: 360,
      minHeight: 316,
      maxWidth: 720,
      maxHeight: 520,
    });
    expect(setSizeConstraints.mock.invocationCallOrder[0]).toBeLessThan(
      setSize.mock.invocationCallOrder[0],
    );
  });

  it("adds a 17px anchor gap after measured content", async () => {
    await syncWidgetWindowHeight(1, 400);

    expect(setSize).toHaveBeenCalledWith(expect.objectContaining({ height: 417 }));
    expect(setSizeConstraints).toHaveBeenCalledWith({
      minWidth: 360,
      minHeight: 244,
      maxWidth: 720,
      maxHeight: 520,
    });
  });

  it("keeps the vertical resize range open when content reaches the maximum", async () => {
    await syncWidgetWindowHeight(1, 999);

    expect(setSize).toHaveBeenCalledWith(expect.objectContaining({ height: 520 }));
    expect(setSizeConstraints).toHaveBeenCalledWith({
      minWidth: 360,
      minHeight: 244,
      maxWidth: 720,
      maxHeight: 520,
    });
  });
});
