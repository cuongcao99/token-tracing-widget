import { isProviderId, providerOrder, type ProviderId } from "../provider";
import { isThemeId, type ThemeId } from "../theme";
import type { VisibleProviderSetting } from "./widget-settings";
import { hasExactKeys, isRecord } from "./validation";

export interface SourceEnabledSetting {
  provider: ProviderId;
  enabled: boolean;
}

export interface WidgetSettingsPreview {
  darkMode: boolean;
  theme: ThemeId;
  visibleProviders: VisibleProviderSetting[];
  sourceEnabled: SourceEnabledSetting[];
}

const previewKeys = ["darkMode", "theme", "visibleProviders", "sourceEnabled"] as const;
const visibleProviderKeys = ["provider", "visible"] as const;
const sourceEnabledKeys = ["provider", "enabled"] as const;

export function parseWidgetSettingsPreview(
  value: unknown,
): WidgetSettingsPreview | null {
  if (!isRecord(value) || !hasExactKeys(value, previewKeys)) return null;
  if (
    typeof value.darkMode !== "boolean" ||
    !isThemeId(value.theme) ||
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

  return {
    darkMode: value.darkMode,
    theme: value.theme,
    visibleProviders,
    sourceEnabled,
  };
}
