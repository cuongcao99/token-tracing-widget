import { act, cleanup, renderHook, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  useSettingsPersistence,
  type UseSettingsPersistenceResult,
} from "../../hooks/useSettingsPersistence";
import type { SourceSettings } from "../../lib/source-settings";
import type { WidgetSettingsSnapshot } from "../../lib/widget-settings";
import type { WidgetSettingsPreview } from "../../lib/widget-settings-preview";

const mocks = vi.hoisted(() => ({
  emitPreview: vi.fn(),
  updateWidgetSettings: vi.fn(),
  updateSourceSettings: vi.fn(),
  saveUpdateSettings: vi.fn(),
}));

vi.mock("../../lib/widget-settings-preview", () => ({ emitWidgetSettingsPreview: mocks.emitPreview }));
vi.mock("../../lib/widget-settings", () => ({ updateWidgetSettings: mocks.updateWidgetSettings }));
vi.mock("../../lib/source-settings", () => ({ updateSourceSettings: mocks.updateSourceSettings }));
vi.mock("../../lib/update-settings", () => ({ saveUpdateSettings: mocks.saveUpdateSettings }));

const widgetSnapshot: WidgetSettingsSnapshot = {
  darkMode: true,
  theme: "claude",
  visibleProviders: [
    { provider: "claude", visible: true },
    { provider: "codex", visible: true },
  ],
};
const sourceSettings: SourceSettings = {
  provider: "codex",
  enabled: false,
  windowsRoot: null,
  wslRoot: null,
};
const preview: WidgetSettingsPreview = {
  darkMode: false,
  theme: "claude",
  visibleProviders: [
    { provider: "claude", visible: true },
    { provider: "codex", visible: false },
  ],
  sourceEnabled: [
    { provider: "claude", enabled: true },
    { provider: "codex", enabled: false },
  ],
};
const updateSettings = { autoUpdate: true };

function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((resolvePromise) => {
    resolve = resolvePromise;
  });
  return { promise, resolve };
}

function renderPersistence(onError = vi.fn()) {
  return {
    onError,
    ...renderHook(() => useSettingsPersistence(onError)),
  };
}

beforeEach(() => {
  mocks.emitPreview.mockResolvedValue(undefined);
  mocks.updateWidgetSettings.mockResolvedValue(widgetSnapshot);
  mocks.updateSourceSettings.mockResolvedValue({ sources: [] });
  mocks.saveUpdateSettings.mockResolvedValue(updateSettings);
});

afterEach(() => {
  cleanup();
  vi.clearAllMocks();
});

describe("useSettingsPersistence", () => {
  it("serializes writes and keeps flush pending until the newest write settles", async () => {
    const first = deferred<WidgetSettingsSnapshot>();
    const second = deferred<WidgetSettingsSnapshot>();
    mocks.updateWidgetSettings
      .mockImplementationOnce(() => first.promise)
      .mockImplementationOnce(() => second.promise);
    const { result } = renderPersistence();

    act(() => {
      result.current.saveWidget(widgetSnapshot);
      result.current.saveWidget({ ...widgetSnapshot, darkMode: false });
    });
    await waitFor(() => expect(mocks.updateWidgetSettings).toHaveBeenCalledTimes(1));
    expect(mocks.updateWidgetSettings).toHaveBeenLastCalledWith(widgetSnapshot);

    let flushed = false;
    const pendingFlush = result.current.flush().then(() => {
      flushed = true;
    });
    first.resolve(widgetSnapshot);
    await waitFor(() => expect(mocks.updateWidgetSettings).toHaveBeenCalledTimes(2));
    expect(flushed).toBe(false);

    second.resolve({ ...widgetSnapshot, darkMode: false });
    await pendingFlush;
    expect(flushed).toBe(true);
  });

  it("tracks a deferred preview in flush without reordering it", async () => {
    const pendingPreview = deferred<void>();
    mocks.emitPreview.mockReturnValueOnce(pendingPreview.promise);
    const { result } = renderPersistence();

    act(() => result.current.sendPreview(preview));
    let flushed = false;
    const pendingFlush = result.current.flush().then(() => {
      flushed = true;
    });
    await Promise.resolve();
    expect(mocks.emitPreview).toHaveBeenCalledWith(preview);
    expect(flushed).toBe(false);

    pendingPreview.resolve();
    await pendingFlush;
    expect(flushed).toBe(true);
  });

  it("reports native write errors through the provided callback", async () => {
    const onError = vi.fn();
    mocks.updateSourceSettings.mockRejectedValueOnce(new Error("settings_write"));
    const { result } = renderPersistence(onError);

    act(() => result.current.saveSource(sourceSettings));
    await result.current.flush();
    await waitFor(() => expect(onError).toHaveBeenCalledWith("Could not save settings."));
  });

  it("serializes automatic-update preference with the other settings writes", async () => {
    const first = deferred<WidgetSettingsSnapshot>();
    mocks.updateWidgetSettings.mockReturnValueOnce(first.promise);
    const { result } = renderPersistence();

    act(() => {
      result.current.saveWidget(widgetSnapshot);
      result.current.saveUpdate(updateSettings);
    });
    await waitFor(() => expect(mocks.updateWidgetSettings).toHaveBeenCalledTimes(1));
    expect(mocks.saveUpdateSettings).not.toHaveBeenCalled();

    first.resolve(widgetSnapshot);
    await waitFor(() => expect(mocks.saveUpdateSettings).toHaveBeenCalledWith(updateSettings));
    await result.current.flush();
  });

  it("continues the FIFO queue after a failed write", async () => {
    const onError = vi.fn();
    const nextSnapshot = { ...widgetSnapshot, darkMode: false };
    mocks.updateWidgetSettings
      .mockRejectedValueOnce(new Error("settings_write"))
      .mockResolvedValueOnce(nextSnapshot);
    const { result } = renderPersistence(onError);

    act(() => {
      result.current.saveWidget(widgetSnapshot);
      result.current.saveWidget(nextSnapshot);
    });

    await result.current.flush();
    expect(mocks.updateWidgetSettings).toHaveBeenCalledTimes(2);
    expect(mocks.updateWidgetSettings).toHaveBeenLastCalledWith(nextSnapshot);
    await waitFor(() => expect(onError).toHaveBeenCalledWith("Could not save settings."));
  });

  it("ignores new work after cleanup while allowing started work to settle", async () => {
    const started = deferred<WidgetSettingsSnapshot>();
    const queued = deferred<WidgetSettingsSnapshot>();
    const onError = vi.fn();
    mocks.updateWidgetSettings
      .mockReturnValueOnce(started.promise)
      .mockReturnValueOnce(queued.promise);
    const rendered = renderPersistence(onError);

    act(() => {
      rendered.result.current.saveWidget(widgetSnapshot);
      rendered.result.current.saveWidget({ ...widgetSnapshot, darkMode: false });
    });
    await waitFor(() => expect(mocks.updateWidgetSettings).toHaveBeenCalledTimes(1));
    rendered.unmount();

    act(() => {
      rendered.result.current.saveWidget(widgetSnapshot);
      rendered.result.current.saveSource(sourceSettings);
      rendered.result.current.sendPreview(preview);
    });
    expect(mocks.updateWidgetSettings).toHaveBeenCalledTimes(1);
    expect(mocks.updateSourceSettings).not.toHaveBeenCalled();
    expect(mocks.emitPreview).not.toHaveBeenCalled();

    started.resolve(widgetSnapshot);
    await waitFor(() => expect(mocks.updateWidgetSettings).toHaveBeenCalledTimes(1));
    expect(onError).not.toHaveBeenCalled();
  });
});
