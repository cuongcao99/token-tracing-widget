import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import {
  getWidgetSettings,
  listenForWidgetSettings,
  parseWidgetSettings,
  updateWidgetSettings,
  type WidgetSettingsSnapshot,
} from "../../lib/widget-settings";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn() }));

const validSettings: WidgetSettingsSnapshot = {
  darkMode: true,
  theme: "claude",
  visibleProviders: [
    { provider: "claude", visible: true },
    { provider: "codex", visible: false },
  ],
};

beforeEach(() => {
  vi.mocked(invoke).mockReset();
  vi.mocked(listen).mockReset();
});

describe("widget settings bridge", () => {
  it("gets and validates persisted widget settings", async () => {
    vi.mocked(invoke).mockResolvedValue(validSettings);

    await expect(getWidgetSettings()).resolves.toEqual(validSettings);
    expect(invoke).toHaveBeenCalledWith("get_widget_settings");
  });

  it("sends only the typed settings payload", async () => {
    vi.mocked(invoke).mockResolvedValue(validSettings);

    await updateWidgetSettings(validSettings);

    expect(invoke).toHaveBeenCalledWith("update_widget_settings", {
      settings: validSettings,
    });
  });

  it("rejects duplicate providers and forbidden fields", () => {
    expect(
      parseWidgetSettings({
        ...validSettings,
        visibleProviders: [
          { provider: "claude", visible: true },
          { provider: "claude", visible: false },
        ],
      }),
    ).toBeNull();
    expect(
      parseWidgetSettings({ ...validSettings, rawRecord: "secret" }),
    ).toBeNull();
    expect(
      parseWidgetSettings({ ...validSettings, theme: "private-theme" }),
    ).toBeNull();
  });

  it("forwards only valid preference-changed payloads", async () => {
    const onSettings = vi.fn();
    const stop = vi.fn();
    let emit: ((payload: unknown) => void) | undefined;
    vi.mocked(listen).mockImplementation(async (_event, handler) => {
      emit = (payload) => handler({ payload } as Parameters<typeof handler>[0]);
      return stop;
    });

    await listenForWidgetSettings(onSettings);
    emit!(validSettings);
    emit!({ ...validSettings, sourceRoot: "private" });

    expect(onSettings).toHaveBeenCalledTimes(1);
    expect(onSettings).toHaveBeenCalledWith(validSettings);
  });
});
