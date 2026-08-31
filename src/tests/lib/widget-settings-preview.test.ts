import { beforeEach, describe, expect, it, vi } from "vitest";
import { emit, listen } from "@tauri-apps/api/event";
import {
  emitWidgetSettingsPreview,
  listenForWidgetSettingsPreview,
  parseWidgetSettingsPreview,
  WIDGET_SETTINGS_PREVIEW_CHANGED_EVENT,
} from "../../lib/widget-settings-preview";
import type { WidgetSettingsPreview } from "../../lib/widget-settings-preview";

vi.mock("@tauri-apps/api/event", () => ({
  emit: vi.fn(),
  listen: vi.fn(),
}));

beforeEach(() => {
  vi.mocked(emit).mockReset();
  vi.mocked(listen).mockReset();
});

describe("widget settings preview bridge", () => {
  const preview: WidgetSettingsPreview = {
    darkMode: false,
    visibleProviders: [
      { provider: "claude", visible: true },
      { provider: "codex", visible: false },
    ],
    sourceEnabled: [
      { provider: "claude", enabled: true },
      { provider: "codex", enabled: false },
    ],
  };

  it("emits only the typed full settings preview payload", async () => {
    await emitWidgetSettingsPreview(preview);

    expect(emit).toHaveBeenCalledWith(
      WIDGET_SETTINGS_PREVIEW_CHANGED_EVENT,
      preview,
    );
  });

  it("rejects preview payloads containing fields outside the contract", () => {
    expect(parseWidgetSettingsPreview(preview)).toEqual(preview);
    expect(parseWidgetSettingsPreview({ darkMode: true })).toBeNull();
    expect(
      parseWidgetSettingsPreview({
        ...preview,
        sourceRoot: "private",
      }),
    ).toBeNull();
    expect(
      parseWidgetSettingsPreview({
        ...preview,
        visibleProviders: [
          { provider: "claude", visible: true },
          { provider: "claude", visible: false },
        ],
      }),
    ).toBeNull();
    expect(
      parseWidgetSettingsPreview({
        ...preview,
        sourceEnabled: [
          { provider: "claude", enabled: true },
          { provider: "codex", enabled: "false" },
        ],
      }),
    ).toBeNull();
  });

  it("forwards only valid preview events", async () => {
    const onPreview = vi.fn();
    const stop = vi.fn();
    let dispatch: ((payload: unknown) => void) | undefined;
    vi.mocked(listen).mockImplementation(async (_event, handler) => {
      dispatch = (payload) => handler({ payload } as Parameters<typeof handler>[0]);
      return stop;
    });

    await listenForWidgetSettingsPreview(onPreview);
    dispatch!(preview);
    dispatch!({ ...preview, rawRecord: "secret" });

    expect(onPreview).toHaveBeenCalledTimes(1);
    expect(onPreview).toHaveBeenCalledWith(preview);
  });
});
