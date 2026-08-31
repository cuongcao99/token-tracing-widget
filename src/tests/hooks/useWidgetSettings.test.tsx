import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { useWidgetSettings } from "../../hooks/useWidgetSettings";
import type { WidgetSettingsSnapshot } from "../../lib/widget-settings";
import type { WidgetSettingsPreview } from "../../lib/widget-settings-preview";

const {
  getWidgetSettings,
  listenForWidgetSettings,
  listenForWidgetSettingsPreview,
} = vi.hoisted(() => ({
  getWidgetSettings: vi.fn(),
  listenForWidgetSettings: vi.fn(),
  listenForWidgetSettingsPreview: vi.fn(),
}));

vi.mock("../../lib/widget-settings", () => ({
  getWidgetSettings,
  listenForWidgetSettings,
}));
vi.mock("../../lib/widget-settings-preview", () => ({
  listenForWidgetSettingsPreview,
}));

const settings: WidgetSettingsSnapshot = {
  darkMode: false,
  visibleProviders: [
    { provider: "claude", visible: true },
    { provider: "codex", visible: false },
  ],
};

beforeEach(() => {
  getWidgetSettings.mockReset();
  listenForWidgetSettings.mockReset();
  listenForWidgetSettingsPreview.mockReset();
});

describe("useWidgetSettings", () => {
  it("subscribes before loading persisted settings and cleans up on unmount", async () => {
    const calls: string[] = [];
    const unlisten = vi.fn();
    listenForWidgetSettings.mockImplementation(async () => {
      calls.push("listen");
      return unlisten;
    });
    getWidgetSettings.mockImplementation(async () => {
      calls.push("get");
      return settings;
    });

    const rendered = renderHook(() => useWidgetSettings());
    await waitFor(() => expect(rendered.result.current.settings).toEqual(settings));

    expect(calls).toEqual(["listen", "get"]);
    expect(rendered.result.current.loading).toBe(false);
    rendered.unmount();
    expect(unlisten).toHaveBeenCalledTimes(1);
  });

  it("applies a transient preview without changing the persisted baseline", async () => {
    const unlisten = vi.fn();
    const unlistenPreview = vi.fn();
    let onSettings: ((nextSettings: WidgetSettingsSnapshot) => void) | undefined;
    let onPreview: ((preview: WidgetSettingsPreview) => void) | undefined;
    listenForWidgetSettings.mockImplementation(async (handler) => {
      onSettings = handler;
      return unlisten;
    });
    listenForWidgetSettingsPreview.mockImplementation(async (handler) => {
      onPreview = handler;
      return unlistenPreview;
    });
    getWidgetSettings.mockResolvedValue({ ...settings, darkMode: true });

    const rendered = renderHook(() => useWidgetSettings());
    await waitFor(() => expect(rendered.result.current.settings.darkMode).toBe(true));

    act(() =>
      onPreview!({
        darkMode: false,
        visibleProviders: [
          { provider: "claude", visible: false },
          { provider: "codex", visible: true },
        ],
        sourceEnabled: [
          { provider: "claude", enabled: false },
          { provider: "codex", enabled: true },
        ],
      }),
    );
    expect(rendered.result.current.settings.darkMode).toBe(false);
    expect(rendered.result.current.settings.visibleProviders).toEqual([
      { provider: "claude", visible: false },
      { provider: "codex", visible: true },
    ]);
    expect(rendered.result.current.previewSourceEnabled).toEqual({
      claude: false,
      codex: true,
    });
    expect(rendered.result.current.persistedSettings.darkMode).toBe(true);

    act(() => onSettings!({ ...settings, darkMode: false }));
    expect(rendered.result.current.settings.darkMode).toBe(false);
    expect(rendered.result.current.persistedSettings.darkMode).toBe(false);
    expect(rendered.result.current.previewSourceEnabled).toBeNull();

    rendered.unmount();
    expect(unlistenPreview).toHaveBeenCalledTimes(1);
  });

  it("starts with dark/both-visible defaults and exposes a sanitized load error", async () => {
    listenForWidgetSettings.mockResolvedValue(vi.fn());
    getWidgetSettings.mockRejectedValue(new Error("widget_settings_unavailable"));

    const { result } = renderHook(() => useWidgetSettings());
    expect(result.current.settings.darkMode).toBe(true);
    expect(result.current.settings.visibleProviders).toHaveLength(2);
    await waitFor(() => expect(result.current.error?.message).toBe("widget_settings_unavailable"));
  });
});
