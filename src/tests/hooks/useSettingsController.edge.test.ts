import { act, cleanup, renderHook, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import useSettingsController from "../../hooks/useSettingsController";
import type { SourceSettingsSnapshot } from "../../lib/source-settings";
import type { WidgetSettingsSnapshot } from "../../lib/widget-settings";

const mocks = vi.hoisted(() => ({
  useWidgetSettings: vi.fn(),
  getSourceSettings: vi.fn(),
  pickSourceRoot: vi.fn(),
  updateSourceSettings: vi.fn(),
  updateWidgetSettings: vi.fn(),
  emitPreview: vi.fn(),
}));

vi.mock("../../hooks/useWidgetSettings", () => ({ useWidgetSettings: mocks.useWidgetSettings }));
vi.mock("../../lib/source-settings", () => ({
  getSourceSettings: mocks.getSourceSettings,
  pickSourceRoot: mocks.pickSourceRoot,
  updateSourceSettings: mocks.updateSourceSettings,
}));
vi.mock("../../lib/widget-settings", () => ({
  updateWidgetSettings: mocks.updateWidgetSettings,
}));
vi.mock("../../lib/widget-settings-preview", () => ({
  emitWidgetSettingsPreview: mocks.emitPreview,
}));
vi.mock("../../lib/window-actions", () => ({
  closeCurrentWindow: vi.fn(),
}));

const widgetSettings: WidgetSettingsSnapshot = {
  darkMode: true,
  theme: "claude",
  visibleProviders: [
    { provider: "claude", visible: true },
    { provider: "codex", visible: true },
  ],
};

const sourceSnapshot: SourceSettingsSnapshot = {
  sources: [
    { provider: "claude", enabled: true, rootOverride: null },
    { provider: "codex", enabled: true, rootOverride: null },
  ],
};

beforeEach(() => {
  mocks.useWidgetSettings.mockReturnValue({
    settings: widgetSettings,
    persistedSettings: widgetSettings,
    previewSourceEnabled: null,
    loading: false,
    error: null,
  });
  mocks.getSourceSettings.mockReturnValue(new Promise<SourceSettingsSnapshot>(() => undefined));
  mocks.pickSourceRoot.mockResolvedValue(null);
  mocks.updateWidgetSettings.mockResolvedValue(widgetSettings);
  mocks.updateSourceSettings.mockResolvedValue(sourceSnapshot);
  mocks.emitPreview.mockResolvedValue(undefined);
});

afterEach(() => {
  cleanup();
  vi.clearAllMocks();
});

describe("useSettingsController edge behavior", () => {
  it("does not preview an edit while source settings are still unavailable", async () => {
    const { result } = renderHook(() => useSettingsController());

    act(() => result.current.onDarkModeToggle(false));

    expect(mocks.emitPreview).not.toHaveBeenCalled();
    await waitFor(() => expect(mocks.updateWidgetSettings).toHaveBeenCalledWith({
      theme: "claude",
      darkMode: false,
      visibleProviders: widgetSettings.visibleProviders,
    }));
  });

  it("does not write settings when the native source picker is cancelled", async () => {
    const { result } = renderHook(() => useSettingsController());
    await act(async () => {
      await result.current.onSourceRootChoose("codex");
    });

    expect(mocks.pickSourceRoot).toHaveBeenCalledWith("codex");
    expect(mocks.updateWidgetSettings).not.toHaveBeenCalled();
    expect(mocks.updateSourceSettings).not.toHaveBeenCalled();
    expect(mocks.emitPreview).not.toHaveBeenCalled();
  });
});
