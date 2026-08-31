import { useEffect, useMemo, useState } from "react";
import type { UnlistenFn } from "@tauri-apps/api/event";
import {
  getWidgetSettings,
  listenForWidgetSettings,
  type WidgetSettingsSnapshot,
} from "../lib/widget-settings";
import {
  listenForWidgetSettingsPreview,
  type WidgetSettingsPreview,
} from "../lib/widget-settings-preview";
import { providerOrder, type ProviderId } from "../lib/provider";

export const defaultWidgetSettings: WidgetSettingsSnapshot = {
  darkMode: true,
  visibleProviders: providerOrder.map((provider) => ({
    provider,
    visible: true,
  })),
};

export interface UseWidgetSettingsResult {
  settings: WidgetSettingsSnapshot;
  persistedSettings: WidgetSettingsSnapshot;
  previewSourceEnabled: Record<ProviderId, boolean> | null;
  loading: boolean;
  error: Error | null;
}

export function useWidgetSettings(): UseWidgetSettingsResult {
  const [persistedSettings, setPersistedSettings] =
    useState(defaultWidgetSettings);
  const [preview, setPreview] = useState<WidgetSettingsPreview | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<Error | null>(null);

  useEffect(() => {
    let mounted = true;
    let unlisten: UnlistenFn | undefined;
    let unlistenPreview: UnlistenFn | undefined;

    const connect = async () => {
      try {
        const stop = await listenForWidgetSettings((nextSettings) => {
          if (mounted) {
            setPersistedSettings(nextSettings);
            setPreview(null);
            setLoading(false);
            setError(null);
          }
        });
        if (!mounted) {
          void stop();
        } else {
          unlisten = stop;
        }
      } catch {
        // The command below still provides a useful one-shot read.
      }

      try {
        const stop = await listenForWidgetSettingsPreview((preview) => {
          if (mounted) setPreview(preview);
        });
        if (!mounted) {
          void stop();
        } else {
          unlistenPreview = stop;
        }
      } catch {
        // The persisted settings command still provides a useful one-shot read.
      }

      try {
        const initialSettings = await getWidgetSettings();
        if (mounted) {
          setPersistedSettings(initialSettings);
          setError(null);
        }
      } catch (loadError) {
        if (mounted) {
          setError(loadError instanceof Error ? loadError : new Error("widget_settings_unavailable"));
        }
      } finally {
        if (mounted) setLoading(false);
      }
    };

    void connect();
    return () => {
      mounted = false;
      if (unlisten) void unlisten();
      if (unlistenPreview) void unlistenPreview();
    };
  }, []);

  const settings = useMemo(
    () =>
      preview === null
        ? persistedSettings
        : {
            ...persistedSettings,
            darkMode: preview.darkMode,
            visibleProviders: preview.visibleProviders,
          },
    [persistedSettings, preview],
  );

  const previewSourceEnabled = useMemo(() => {
    if (!preview) return null;
    const values = {} as Record<ProviderId, boolean>;
    for (const source of preview.sourceEnabled) {
      values[source.provider] = source.enabled;
    }
    return values;
  }, [preview]);

  return {
    settings,
    persistedSettings,
    previewSourceEnabled,
    loading,
    error,
  };
}
