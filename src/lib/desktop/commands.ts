import { invoke } from "@tauri-apps/api/core";
import type { ProviderId } from "../provider";
import type { SourcePlatform, SourceSettings } from "../contracts/source-settings";
import type { UpdateSettingsSnapshot } from "../contracts/update-settings";
import type { WidgetSettingsSnapshot } from "../contracts/widget-settings";

export const GET_USAGE_SUMMARY_COMMAND = "get_usage_summary";
export const GET_WIDGET_SETTINGS_COMMAND = "get_widget_settings";
export const UPDATE_WIDGET_SETTINGS_COMMAND = "update_widget_settings";
export const GET_SOURCE_SETTINGS_COMMAND = "get_source_settings";
export const PICK_SOURCE_ROOT_COMMAND = "pick_source_root";
export const UPDATE_SOURCE_SETTINGS_COMMAND = "update_source_settings";
export const GET_UPDATE_SETTINGS_COMMAND = "get_update_settings";
export const SAVE_UPDATE_SETTINGS_COMMAND = "save_update_settings";

export function invokeUsageSummary(): Promise<unknown> {
  return invoke<unknown>(GET_USAGE_SUMMARY_COMMAND);
}

export function invokeWidgetSettings(): Promise<unknown> {
  return invoke<unknown>(GET_WIDGET_SETTINGS_COMMAND);
}

export function invokeUpdateWidgetSettings(
  settings: WidgetSettingsSnapshot,
): Promise<unknown> {
  return invoke<unknown>(UPDATE_WIDGET_SETTINGS_COMMAND, { settings });
}

export function invokeSourceSettings(): Promise<unknown> {
  return invoke<unknown>(GET_SOURCE_SETTINGS_COMMAND);
}

export function invokePickSourceRoot(
  provider: ProviderId,
  platform: SourcePlatform,
): Promise<unknown> {
  return invoke<unknown>(PICK_SOURCE_ROOT_COMMAND, { provider, platform });
}

export function invokeUpdateSourceSettings(
  settings: SourceSettings,
): Promise<unknown> {
  return invoke<unknown>(UPDATE_SOURCE_SETTINGS_COMMAND, { settings });
}

export function invokeUpdateSettings(): Promise<unknown> {
  return invoke<unknown>(GET_UPDATE_SETTINGS_COMMAND);
}

export function invokeSaveUpdateSettings(
  settings: UpdateSettingsSnapshot,
): Promise<unknown> {
  return invoke<unknown>(SAVE_UPDATE_SETTINGS_COMMAND, { settings });
}
