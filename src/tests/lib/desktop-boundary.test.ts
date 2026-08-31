import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import { emit, listen } from "@tauri-apps/api/event";
import {
  closeCurrentWindow,
  startCurrentWindowDrag,
  startCurrentWindowResize,
} from "../../lib/window-actions";
import {
  emitWidgetSettingsPreview,
  listenForWidgetSettingsPreview,
} from "../../lib/widget-settings-preview";
import {
  getSourceSettings,
  pickSourceRoot,
  updateSourceSettings,
} from "../../lib/source-settings";
import {
  getUsageSummary,
  listenForUsageSummary,
} from "../../lib/usage-summary";
import {
  getWidgetSettings,
  listenForWidgetSettings,
  updateWidgetSettings,
} from "../../lib/widget-settings";
import { parseUsageSummary } from "../../lib/contracts/usage-summary";
import { parseWidgetSettings } from "../../lib/contracts/widget-settings";
import { parseWidgetSettingsPreview } from "../../lib/contracts/widget-settings-preview";
import { parseSourceSettings } from "../../lib/contracts/source-settings";

const {
  getCurrentWindow,
  startDragging,
  startResizeDragging,
  close,
} = vi.hoisted(() => {
  const startDragging = vi.fn().mockResolvedValue(undefined);
  const startResizeDragging = vi.fn().mockResolvedValue(undefined);
  const close = vi.fn().mockResolvedValue(undefined);
  const getCurrentWindow = vi.fn(() => ({
    startDragging,
    startResizeDragging,
    close,
  }));
  return { getCurrentWindow, startDragging, startResizeDragging, close };
});

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/event", () => ({
  emit: vi.fn(),
  listen: vi.fn(),
}));
vi.mock("@tauri-apps/api/window", async () => {
  const actual = await vi.importActual<typeof import("@tauri-apps/api/window")>(
    "@tauri-apps/api/window",
  );
  return { ...actual, getCurrentWindow };
});

const summary = {
  state: "active" as const,
  provider: "Any display label",
  todayTokens: 12,
  sourceHealth: [
    { provider: "claude" as const, state: "detected" },
    { provider: "codex" as const, state: "detected" },
  ],
  providers: [
    { provider: "claude" as const, state: "idle" as const, todayTokens: 4 },
    { provider: "codex" as const, state: "active" as const, todayTokens: 8 },
  ],
};

const widgetSettings = {
  darkMode: true,
  theme: "claude" as const,
  visibleProviders: [
    { provider: "claude" as const, visible: true },
    { provider: "codex" as const, visible: false },
  ],
};

const sourceSnapshot = {
  sources: [
    { provider: "claude" as const, enabled: true, rootOverride: null },
    { provider: "codex" as const, enabled: false, rootOverride: null },
  ],
};

const preview = {
  darkMode: false,
  theme: "claude" as const,
  visibleProviders: widgetSettings.visibleProviders,
  sourceEnabled: [
    { provider: "claude" as const, enabled: true },
    { provider: "codex" as const, enabled: false },
  ],
};

beforeEach(() => {
  vi.mocked(invoke).mockReset();
  vi.mocked(emit).mockReset();
  vi.mocked(listen).mockReset();
  startDragging.mockClear();
  startResizeDragging.mockClear();
  close.mockClear();
});

describe("desktop compatibility boundary", () => {
  it("keeps exact command names and payload shapes", async () => {
    vi.mocked(invoke)
      .mockResolvedValueOnce(summary)
      .mockResolvedValueOnce(widgetSettings)
      .mockResolvedValueOnce(widgetSettings)
      .mockResolvedValueOnce(sourceSnapshot)
      .mockResolvedValueOnce(sourceSnapshot)
      .mockResolvedValueOnce(sourceSnapshot);

    await getUsageSummary();
    await getWidgetSettings();
    await updateWidgetSettings(widgetSettings);
    await getSourceSettings();
    await pickSourceRoot("codex");
    await updateSourceSettings(sourceSnapshot.sources[0]);

    expect(invoke).toHaveBeenNthCalledWith(1, "get_usage_summary");
    expect(invoke).toHaveBeenNthCalledWith(2, "get_widget_settings");
    expect(invoke).toHaveBeenNthCalledWith(3, "update_widget_settings", {
      settings: widgetSettings,
    });
    expect(invoke).toHaveBeenNthCalledWith(4, "get_source_settings");
    expect(invoke).toHaveBeenNthCalledWith(5, "pick_source_root", {
      provider: "codex",
    });
    expect(invoke).toHaveBeenNthCalledWith(6, "update_source_settings", {
      settings: sourceSnapshot.sources[0],
    });
  });

  it("keeps exact event names and unlisten cleanup", async () => {
    const stop = vi.fn();
    vi.mocked(listen).mockImplementation(async (event, handler) => {
      expect(event).toMatch(
        /^(usage-summary-changed|widget-settings-changed|widget-settings-preview-changed)$/,
      );
      return stop;
    });

    const summaryStop = await listenForUsageSummary(vi.fn());
    const settingsStop = await listenForWidgetSettings(vi.fn());
    const previewStop = await listenForWidgetSettingsPreview(vi.fn());
    await summaryStop();
    await settingsStop();
    await previewStop();

    expect(listen).toHaveBeenNthCalledWith(
      1,
      "usage-summary-changed",
      expect.any(Function),
    );
    expect(listen).toHaveBeenNthCalledWith(
      2,
      "widget-settings-changed",
      expect.any(Function),
    );
    expect(listen).toHaveBeenNthCalledWith(
      3,
      "widget-settings-preview-changed",
      expect.any(Function),
    );
    expect(stop).toHaveBeenCalledTimes(3);
  });

  it("forwards the validated preview and native window actions", async () => {
    await emitWidgetSettingsPreview(preview);
    await startCurrentWindowDrag();
    await startCurrentWindowResize("SouthEast");
    await closeCurrentWindow();

    expect(emit).toHaveBeenCalledWith("widget-settings-preview-changed", preview);
    expect(startDragging).toHaveBeenCalledTimes(1);
    expect(startResizeDragging).toHaveBeenCalledWith("SouthEast");
    expect(close).toHaveBeenCalledTimes(1);
  });

  it("rejects unsafe and invalid contract payloads", () => {
    expect(parseUsageSummary(summary)).toEqual(summary);
    expect(parseUsageSummary({ ...summary, prompt: "secret" })).toBeNull();
    expect(parseUsageSummary({ ...summary, todayTokens: -1 })).toBeNull();
    expect(parseUsageSummary({ ...summary, lastUpdatedAt: "not-a-date" })).toBeNull();
    expect(
      parseUsageSummary({
        ...summary,
        providers: [
          ...summary.providers.slice(0, 1),
          { provider: "other", state: "active", todayTokens: 1 },
        ],
      }),
    ).toBeNull();
    expect(parseWidgetSettings({ ...widgetSettings, theme: "private" })).toBeNull();
    expect(
      parseWidgetSettings({
        ...widgetSettings,
        visibleProviders: [
          { provider: "claude", visible: true },
          { provider: "claude", visible: false },
        ],
      }),
    ).toBeNull();
    expect(parseWidgetSettingsPreview({ ...preview, rawRecord: "secret" })).toBeNull();
    expect(
      parseSourceSettings({
        sources: [
          { provider: "claude", enabled: true, rootOverride: { path: "private" } },
          sourceSnapshot.sources[1],
        ],
      }),
    ).toBeNull();
  });
});
