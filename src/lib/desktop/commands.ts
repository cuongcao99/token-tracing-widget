import { invoke } from "@tauri-apps/api/core";
import type { ProviderId } from "../provider";
import type { SourceSettings } from "../contracts/source-settings";
import type { WidgetSettingsSnapshot } from "../contracts/widget-settings";

export const GET_USAGE_SUMMARY_COMMAND = "get_usage_summary";
export const GET_WIDGET_SETTINGS_COMMAND = "get_widget_settings";
export const UPDATE_WIDGET_SETTINGS_COMMAND = "update_widget_settings";
export const GET_SOURCE_SETTINGS_COMMAND = "get_source_settings";
export const PICK_SOURCE_ROOT_COMMAND = "pick_source_root";
export const UPDATE_SOURCE_SETTINGS_COMMAND = "update_source_settings";

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
): Promise<unknown> {
  return invoke<unknown>(PICK_SOURCE_ROOT_COMMAND, { provider });
}

export function invokeUpdateSourceSettings(
  settings: SourceSettings,
): Promise<unknown> {
  return invoke<unknown>(UPDATE_SOURCE_SETTINGS_COMMAND, { settings });
}
