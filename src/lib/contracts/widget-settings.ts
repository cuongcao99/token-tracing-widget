import { isProviderId, providerOrder, type ProviderId } from "../provider";
import { isThemeId, type ThemeId } from "../theme";
import { hasExactKeys, isRecord } from "./validation";

export interface VisibleProviderSetting {
  provider: ProviderId;
  visible: boolean;
}

export interface WidgetSettingsSnapshot {
  visibleProviders: VisibleProviderSetting[];
  darkMode: boolean;
  theme: ThemeId;
}

const snapshotKeys = ["visibleProviders", "darkMode", "theme"] as const;
const settingKeys = ["provider", "visible"] as const;

export function parseWidgetSettings(
  value: unknown,
): WidgetSettingsSnapshot | null {
  if (!isRecord(value) || !hasExactKeys(value, snapshotKeys)) return null;
  if (
    typeof value.darkMode !== "boolean" ||
    !isThemeId(value.theme) ||
    !Array.isArray(value.visibleProviders) ||
    value.visibleProviders.length !== providerOrder.length
  ) {
    return null;
  }

  const seen = new Set<ProviderId>();
  const visibleProviders: VisibleProviderSetting[] = [];
  for (const entry of value.visibleProviders) {
    if (
      !isRecord(entry) ||
      !hasExactKeys(entry, settingKeys) ||
      !isProviderId(entry.provider) ||
      seen.has(entry.provider) ||
      typeof entry.visible !== "boolean"
    ) {
      return null;
    }
    seen.add(entry.provider);
    visibleProviders.push({ provider: entry.provider, visible: entry.visible });
  }

  if (seen.size !== providerOrder.length) return null;
  return { darkMode: value.darkMode, theme: value.theme, visibleProviders };
}
