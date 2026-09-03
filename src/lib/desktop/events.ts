import {
  emit as tauriEmit,
  listen as tauriListen,
} from "@tauri-apps/api/event";

export type UnlistenFn = () => void;

export const USAGE_SUMMARY_CHANGED_EVENT = "usage-summary-changed";
export const WIDGET_SETTINGS_CHANGED_EVENT = "widget-settings-changed";
export const WIDGET_SETTINGS_PREVIEW_CHANGED_EVENT =
  "widget-settings-preview-changed";

export function listenForUsageSummaryEvent(
  onPayload: (payload: unknown) => void,
): Promise<UnlistenFn> {
  return tauriListen<unknown>(USAGE_SUMMARY_CHANGED_EVENT, (event) => {
    onPayload(event.payload);
  });
}

export function listenForWidgetSettingsEvent(
  onPayload: (payload: unknown) => void,
): Promise<UnlistenFn> {
  return tauriListen<unknown>(WIDGET_SETTINGS_CHANGED_EVENT, (event) => {
    onPayload(event.payload);
  });
}

export function emitWidgetSettingsPreviewEvent(
  preview: unknown,
): Promise<void> {
  return tauriEmit(WIDGET_SETTINGS_PREVIEW_CHANGED_EVENT, preview);
}

export function listenForWidgetSettingsPreviewEvent(
  onPayload: (payload: unknown) => void,
): Promise<UnlistenFn> {
  return tauriListen<unknown>(
    WIDGET_SETTINGS_PREVIEW_CHANGED_EVENT,
    (event) => {
      onPayload(event.payload);
    },
  );
}
