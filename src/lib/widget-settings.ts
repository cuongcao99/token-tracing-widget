import {
  listenForWidgetSettingsEvent,
  WIDGET_SETTINGS_CHANGED_EVENT,
  type UnlistenFn,
} from "./desktop/events";
import {
  invokeUpdateWidgetSettings,
  invokeWidgetSettings,
} from "./desktop/commands";
import {
  parseWidgetSettings,
  type VisibleProviderSetting,
  type WidgetSettingsSnapshot,
} from "./contracts/widget-settings";

export {
  parseWidgetSettings,
  type VisibleProviderSetting,
  type WidgetSettingsSnapshot,
} from "./contracts/widget-settings";
export { WIDGET_SETTINGS_CHANGED_EVENT } from "./desktop/events";
export type { UnlistenFn } from "./desktop/events";

export async function getWidgetSettings(): Promise<WidgetSettingsSnapshot> {
  const value = await invokeWidgetSettings();
  const settings = parseWidgetSettings(value);
  if (!settings) throw new Error("invalid_widget_settings");
  return settings;
}

export async function updateWidgetSettings(
  settings: WidgetSettingsSnapshot,
): Promise<WidgetSettingsSnapshot> {
  const value = await invokeUpdateWidgetSettings(settings);
  const nextSettings = parseWidgetSettings(value);
  if (!nextSettings) throw new Error("invalid_widget_settings");
  return nextSettings;
}

export function listenForWidgetSettings(
  onSettings: (settings: WidgetSettingsSnapshot) => void,
): Promise<UnlistenFn> {
  return listenForWidgetSettingsEvent((payload) => {
    const settings = parseWidgetSettings(payload);
    if (settings) onSettings(settings);
  });
}
