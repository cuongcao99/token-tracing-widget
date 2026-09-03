import { useEffect, useRef } from "react";
import {
  emitWidgetSettingsPreview,
  type WidgetSettingsPreview,
} from "../lib/widget-settings-preview";
import {
  updateWidgetSettings,
  type WidgetSettingsSnapshot,
} from "../lib/widget-settings";
import {
  updateSourceSettings,
  type SourceSettings,
} from "../lib/source-settings";
import { errorMessage } from "../components/settings/settings-model";

export interface UseSettingsPersistenceResult {
  sendPreview(preview: WidgetSettingsPreview): void;
  saveWidget(snapshot: WidgetSettingsSnapshot): void;
  saveSource(settings: SourceSettings): void;
  flush(): Promise<void>;
}

export function useSettingsPersistence(
  onError: (message: string) => void,
): UseSettingsPersistenceResult {
  const mounted = useRef(true);
  const onErrorRef = useRef(onError);
  const pendingPreview = useRef<Promise<void>>(Promise.resolve());
  const pendingPersistence = useRef<Promise<void>>(Promise.resolve());

  useEffect(() => {
    mounted.current = true;
    return () => {
      mounted.current = false;
    };
  }, []);

  useEffect(() => {
    onErrorRef.current = onError;
  }, [onError]);

  const reportError = (message: string) => {
    if (mounted.current) onErrorRef.current(message);
  };

  const sendPreview = (preview: WidgetSettingsPreview) => {
    if (!mounted.current) return;

    let request: Promise<void>;
    try {
      request = emitWidgetSettingsPreview(preview);
    } catch {
      reportError("Could not preview settings.");
      return;
    }

    const pending = Promise.allSettled([
      pendingPreview.current,
      request,
    ]).then((results) => {
      if (results.some((result) => result.status === "rejected")) {
        throw new Error("preview_failed");
      }
    });
    pendingPreview.current = pending.catch(() => undefined);
    void pending.catch(() => reportError("Could not preview settings."));
  };

  const enqueuePersistence = (operation: () => Promise<unknown>) => {
    if (!mounted.current) return;

    const request = pendingPersistence.current
      .catch(() => undefined)
      .then(() => (mounted.current ? operation() : undefined))
      .then(() => undefined);
    pendingPersistence.current = request.catch(() => undefined);
    void request.catch((persistError) =>
      reportError(errorMessage(persistError)),
    );
  };

  const saveWidget = (snapshot: WidgetSettingsSnapshot) => {
    enqueuePersistence(() => updateWidgetSettings(snapshot));
  };

  const saveSource = (settings: SourceSettings) => {
    enqueuePersistence(() => updateSourceSettings(settings));
  };

  const flush = async () => {
    await pendingPreview.current;
    await pendingPersistence.current;
  };

  return { sendPreview, saveWidget, saveSource, flush };
}

export default useSettingsPersistence;
