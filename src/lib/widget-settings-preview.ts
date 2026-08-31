import {
  emitWidgetSettingsPreviewEvent,
  listenForWidgetSettingsPreviewEvent,
  WIDGET_SETTINGS_PREVIEW_CHANGED_EVENT,
  type UnlistenFn,
} from "./desktop/events";
import {
  parseWidgetSettingsPreview,
  type SourceEnabledSetting,
  type WidgetSettingsPreview,
} from "./contracts/widget-settings-preview";

export {
  parseWidgetSettingsPreview,
  type SourceEnabledSetting,
  type WidgetSettingsPreview,
} from "./contracts/widget-settings-preview";
export type { VisibleProviderSetting } from "./contracts/widget-settings";
export { WIDGET_SETTINGS_PREVIEW_CHANGED_EVENT } from "./desktop/events";
export type { UnlistenFn } from "./desktop/events";

export function emitWidgetSettingsPreview(
  preview: WidgetSettingsPreview,
): Promise<void> {
  const sanitized = parseWidgetSettingsPreview(preview);
  if (!sanitized) {
    return Promise.reject(new Error("invalid_widget_settings_preview"));
  }
  return emitWidgetSettingsPreviewEvent(sanitized);
}

export function listenForWidgetSettingsPreview(
  onPreview: (preview: WidgetSettingsPreview) => void,
): Promise<UnlistenFn> {
  return listenForWidgetSettingsPreviewEvent((payload) => {
    const preview = parseWidgetSettingsPreview(payload);
    if (preview) onPreview(preview);
  });
}
