import { providerOrder, type ProviderId } from "../../lib/provider";
import type {
  SourceSettings,
  SourceSettingsSnapshot,
} from "../../lib/source-settings";
import type {
  WidgetSettingsSnapshot,
} from "../../lib/widget-settings";
import type { WidgetSettingsPreview } from "../../lib/widget-settings-preview";

export type SourceFormValues = Record<ProviderId, SourceSettings>;
export type VisibilityValues = Record<ProviderId, boolean>;

export function sourceValuesFromSnapshot(
  snapshot: SourceSettingsSnapshot,
): SourceFormValues {
  const values = {} as SourceFormValues;
  for (const provider of providerOrder) {
    const source = snapshot.sources.find((entry) => entry.provider === provider);
    if (!source) throw new Error("invalid_source_settings");
    values[provider] = { ...source };
  }
  return values;
}

export function visibilityFromSnapshot(
  snapshot: WidgetSettingsSnapshot,
): VisibilityValues {
  const values = {} as VisibilityValues;
  for (const provider of providerOrder) {
    const setting = snapshot.visibleProviders.find((entry) => entry.provider === provider);
    if (!setting) throw new Error("invalid_widget_settings");
    values[provider] = setting.visible;
  }
  return values;
}

export function createWidgetSettingsPreview(
  darkMode: boolean,
  visible: VisibilityValues,
  sources: SourceFormValues,
): WidgetSettingsPreview {
  return {
    darkMode,
    visibleProviders: providerOrder.map((provider) => ({
      provider,
      visible: visible[provider],
    })),
    sourceEnabled: providerOrder.map((provider) => ({
      provider,
      enabled: sources[provider].enabled,
    })),
  };
}

export function normalizedSourceValues(values: SourceFormValues): SourceFormValues {
  const normalized = {} as SourceFormValues;
  for (const provider of providerOrder) {
    normalized[provider] = {
      ...values[provider],
      rootOverride: values[provider].rootOverride?.trim() || null,
    };
  }
  return normalized;
}

export function errorMessage(error: unknown): string {
  const code = error instanceof Error ? error.message : "";
  if (code.startsWith("invalid_root:")) {
    return "Invalid source root. Use an absolute Windows path or an approved WSL path.";
  }
  if (code === "settings_write" || code === "widget_settings_write") {
    return "Could not save settings.";
  }
  if (code === "settings_refresh") {
    return "Settings were not applied because collection could not refresh.";
  }
  if (code === "invalid_source_settings" || code === "invalid_widget_settings") {
    return "Settings returned an invalid value.";
  }
  return "Settings are unavailable.";
}
