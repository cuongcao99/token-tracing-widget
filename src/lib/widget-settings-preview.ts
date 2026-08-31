import { emit, listen, type UnlistenFn } from "@tauri-apps/api/event";
import { isProviderId, providerOrder, type ProviderId } from "./provider";
import type { VisibleProviderSetting } from "./widget-settings";

export const WIDGET_SETTINGS_PREVIEW_CHANGED_EVENT =
  "widget-settings-preview-changed";

export interface SourceEnabledSetting {
  provider: ProviderId;
  enabled: boolean;
}

export interface WidgetSettingsPreview {
  darkMode: boolean;
  visibleProviders: VisibleProviderSetting[];
  sourceEnabled: SourceEnabledSetting[];
}

const previewKeys = ["darkMode", "visibleProviders", "sourceEnabled"] as const;
const visibleProviderKeys = ["provider", "visible"] as const;
const sourceEnabledKeys = ["provider", "enabled"] as const;

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function hasExactKeys(
  value: Record<string, unknown>,
  allowed: readonly string[],
): boolean {
  const keys = Object.keys(value);
  return (
    keys.length === allowed.length &&
    keys.every((key) => allowed.some((name) => name === key))
  );
}

export function parseWidgetSettingsPreview(
  value: unknown,
): WidgetSettingsPreview | null {
  if (!isRecord(value) || !hasExactKeys(value, previewKeys)) return null;

  if (
    typeof value.darkMode !== "boolean" ||
    !Array.isArray(value.visibleProviders) ||
    value.visibleProviders.length !== providerOrder.length ||
    !Array.isArray(value.sourceEnabled) ||
    value.sourceEnabled.length !== providerOrder.length
  ) {
    return null;
  }

  const visibleProviders: VisibleProviderSetting[] = [];
  const visibleSeen = new Set<ProviderId>();
  for (const entry of value.visibleProviders) {
    if (
      !isRecord(entry) ||
      !hasExactKeys(entry, visibleProviderKeys) ||
      !isProviderId(entry.provider) ||
      visibleSeen.has(entry.provider) ||
      typeof entry.visible !== "boolean"
    ) {
      return null;
    }
    visibleSeen.add(entry.provider);
    visibleProviders.push({ provider: entry.provider, visible: entry.visible });
  }

  const sourceEnabled: SourceEnabledSetting[] = [];
  const sourceSeen = new Set<ProviderId>();
  for (const entry of value.sourceEnabled) {
    if (
      !isRecord(entry) ||
      !hasExactKeys(entry, sourceEnabledKeys) ||
      !isProviderId(entry.provider) ||
      sourceSeen.has(entry.provider) ||
      typeof entry.enabled !== "boolean"
    ) {
      return null;
    }
    sourceSeen.add(entry.provider);
    sourceEnabled.push({ provider: entry.provider, enabled: entry.enabled });
  }

  if (
    visibleSeen.size !== providerOrder.length ||
    sourceSeen.size !== providerOrder.length
  ) {
    return null;
  }

  return { darkMode: value.darkMode, visibleProviders, sourceEnabled };
}

export function emitWidgetSettingsPreview(
  preview: WidgetSettingsPreview,
): Promise<void> {
  const sanitized = parseWidgetSettingsPreview(preview);
  if (!sanitized) {
    return Promise.reject(new Error("invalid_widget_settings_preview"));
  }
  return emit(WIDGET_SETTINGS_PREVIEW_CHANGED_EVENT, sanitized);
}

export function listenForWidgetSettingsPreview(
  onPreview: (preview: WidgetSettingsPreview) => void,
): Promise<UnlistenFn> {
  return listen<unknown>(WIDGET_SETTINGS_PREVIEW_CHANGED_EVENT, (event) => {
    const preview = parseWidgetSettingsPreview(event.payload);
    if (preview) onPreview(preview);
  });
}
