import { providerOrder, type ProviderId } from "./provider";
import type {
  SourceSettings,
  SourceSettingsSnapshot,
} from "./contracts/source-settings";
import type { WidgetSettingsSnapshot } from "./contracts/widget-settings";
import type { WidgetSettingsPreview } from "./contracts/widget-settings-preview";
import type { ThemeId } from "./theme";
import {
  INVALID_SOURCE_ROOT_MESSAGE,
  NATIVE_SETTINGS_COPY,
} from "./desktop/platform-copy";

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
  theme: ThemeId,
  darkMode: boolean,
  visible: VisibilityValues,
  sources: SourceFormValues,
): WidgetSettingsPreview {
  return {
    darkMode,
    theme,
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
  if (code.startsWith("invalid_root:")) return INVALID_SOURCE_ROOT_MESSAGE;
  if (code === "settings_write" || code === "widget_settings_write") {
    return NATIVE_SETTINGS_COPY.saveFailed;
  }
  if (code === "settings_refresh") return NATIVE_SETTINGS_COPY.refreshFailed;
  if (code === "source_root_open") {
    return NATIVE_SETTINGS_COPY.sourceRootOpenFailed;
  }
  if (code === "source_root_invalid") {
    return NATIVE_SETTINGS_COPY.sourceRootInvalid;
  }
  if (code === "source_root_unavailable") {
    return NATIVE_SETTINGS_COPY.sourceRootUnavailable;
  }
  if (code === "hook_config_read" || code === "hook_status_unavailable") {
    return NATIVE_SETTINGS_COPY.traceHookReadFailed;
  }
  if (code === "hook_config_write") {
    return NATIVE_SETTINGS_COPY.traceHookWriteFailed;
  }
  if (code === "hook_config_invalid") {
    return NATIVE_SETTINGS_COPY.traceHookInvalid;
  }
  if (code === "hook_command_unavailable") {
    return NATIVE_SETTINGS_COPY.traceHookUnavailable;
  }
  if (code === "invalid_source_settings" || code === "invalid_widget_settings") {
    return NATIVE_SETTINGS_COPY.invalidSettings;
  }
  return NATIVE_SETTINGS_COPY.unavailable;
}
