import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { isProviderId, providerOrder, type ProviderId } from "./provider";
import { isThemeId, type ThemeId } from "./theme";

export interface VisibleProviderSetting {
  provider: ProviderId;
  visible: boolean;
}

export interface WidgetSettingsSnapshot {
  visibleProviders: VisibleProviderSetting[];
  darkMode: boolean;
  theme: ThemeId;
}

export const WIDGET_SETTINGS_CHANGED_EVENT = "widget-settings-changed";

const snapshotKeys = ["visibleProviders", "darkMode", "theme"] as const;
const settingKeys = ["provider", "visible"] as const;

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

export async function getWidgetSettings(): Promise<WidgetSettingsSnapshot> {
  const value = await invoke<unknown>("get_widget_settings");
  const settings = parseWidgetSettings(value);
  if (!settings) throw new Error("invalid_widget_settings");
  return settings;
}

export async function updateWidgetSettings(
  settings: WidgetSettingsSnapshot,
): Promise<WidgetSettingsSnapshot> {
  const value = await invoke<unknown>("update_widget_settings", { settings });
  const nextSettings = parseWidgetSettings(value);
  if (!nextSettings) throw new Error("invalid_widget_settings");
  return nextSettings;
}

export function listenForWidgetSettings(
  onSettings: (settings: WidgetSettingsSnapshot) => void,
): Promise<UnlistenFn> {
  return listen<unknown>(WIDGET_SETTINGS_CHANGED_EVENT, (event) => {
    const settings = parseWidgetSettings(event.payload);
    if (settings) onSettings(settings);
  });
}
