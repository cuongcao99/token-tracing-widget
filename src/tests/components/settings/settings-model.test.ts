import { describe, expect, it } from "vitest";
import {
  createWidgetSettingsPreview,
  errorMessage,
  normalizedSourceValues,
  sourceValuesFromSnapshot,
  visibilityFromSnapshot,
} from "../../../components/settings/settings-model";
import type { SourceSettingsSnapshot } from "../../../lib/source-settings";
import type { WidgetSettingsSnapshot } from "../../../lib/widget-settings";

const sourceSnapshot: SourceSettingsSnapshot = {
  sources: [
    { provider: "claude", enabled: true, windowsRoot: null, wslRoot: null },
    {
      provider: "codex",
      enabled: false,
      windowsRoot: " C:\\work\\codex ",
      wslRoot: null,
    },
  ],
};

const widgetSnapshot: WidgetSettingsSnapshot = {
  darkMode: true,
  theme: "claude",
  visibleProviders: [
    { provider: "claude", visible: false },
    { provider: "codex", visible: true },
  ],
};

describe("settings model", () => {
  it("maps snapshots into provider-keyed form values", () => {
    const sources = sourceValuesFromSnapshot(sourceSnapshot);
    const visible = visibilityFromSnapshot(widgetSnapshot);

    expect(sources).toEqual({
      claude: { provider: "claude", enabled: true, windowsRoot: null, wslRoot: null },
      codex: {
        provider: "codex",
        enabled: false,
        windowsRoot: " C:\\work\\codex ",
        wslRoot: null,
      },
    });
    expect(visible).toEqual({ claude: false, codex: true });
    expect(sources.claude).not.toBe(sourceSnapshot.sources[0]);
  });

  it("builds the stable preview payload and normalizes source roots", () => {
    const sources = sourceValuesFromSnapshot(sourceSnapshot);
    const visible = visibilityFromSnapshot(widgetSnapshot);

    expect(createWidgetSettingsPreview("claude", false, visible, sources)).toEqual({
      darkMode: false,
      theme: "claude",
      visibleProviders: [
        { provider: "claude", visible: false },
        { provider: "codex", visible: true },
      ],
      sourceEnabled: [
        { provider: "claude", enabled: true },
        { provider: "codex", enabled: false },
      ],
    });

    expect(
      normalizedSourceValues({
        ...sources,
        claude: { ...sources.claude, windowsRoot: "   " },
        codex: { ...sources.codex, windowsRoot: " C:\\custom " },
      }),
    ).toEqual({
      claude: { provider: "claude", enabled: true, windowsRoot: null, wslRoot: null },
      codex: {
        provider: "codex",
        enabled: false,
        windowsRoot: "C:\\custom",
        wslRoot: null,
      },
    });
  });

  it("rejects incomplete snapshots and maps known errors safely", () => {
    expect(() =>
      sourceValuesFromSnapshot({
        sources: [{ provider: "claude", enabled: true, windowsRoot: null, wslRoot: null }],
      }),
    ).toThrowError("invalid_source_settings");
    expect(() =>
      visibilityFromSnapshot({
        visibleProviders: [{ provider: "claude", visible: true }],
        darkMode: false,
        theme: "claude",
      }),
    ).toThrowError("invalid_widget_settings");

    expect(errorMessage(new Error("invalid_root:C:\\private"))).toBe(
      "Invalid source root. Use an absolute Windows path or an approved WSL path.",
    );
    expect(errorMessage(new Error("settings_refresh"))).toBe(
      "Settings were not applied because collection could not refresh.",
    );
    expect(errorMessage(new Error("source_root_invalid"))).toBe(
      "That folder cannot be used as a source.",
    );
    expect(errorMessage(new Error("unknown"))).toBe("Settings are unavailable.");
  });
});
