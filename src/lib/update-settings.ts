import {
  invokeSaveUpdateSettings,
  invokeUpdateSettings,
} from "./desktop/commands";
import {
  parseUpdateSettings,
  type UpdateSettingsSnapshot,
} from "./contracts/update-settings";

export { parseUpdateSettings, type UpdateSettingsSnapshot } from "./contracts/update-settings";

export async function getUpdateSettings(): Promise<UpdateSettingsSnapshot> {
  const value = await invokeUpdateSettings();
  const settings = parseUpdateSettings(value);
  if (!settings) throw new Error("invalid_update_settings");
  return settings;
}

export async function saveUpdateSettings(
  settings: UpdateSettingsSnapshot,
): Promise<UpdateSettingsSnapshot> {
  const value = await invokeSaveUpdateSettings(settings);
  const nextSettings = parseUpdateSettings(value);
  if (!nextSettings) throw new Error("invalid_update_settings");
  return nextSettings;
}
