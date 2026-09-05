import { useEffect, useState } from "react";
import { useSettingsPersistence } from "./useSettingsPersistence";
import { useWidgetSettings } from "./useWidgetSettings";
import { closeCurrentWindow } from "../lib/window-actions";
import { providerOrder, type ProviderId } from "../lib/provider";
import type { ThemeId } from "../lib/theme";
import {
  getSourceSettings,
  pickSourceRoot,
  type SourceSettings,
  type SourcePlatform,
} from "../lib/source-settings";
import { getUpdateSettings } from "../lib/update-settings";
import type { WidgetSettingsSnapshot } from "../lib/widget-settings";
import type { UpdateSettingsSnapshot } from "../lib/update-settings";
import {
  createWidgetSettingsPreview,
  errorMessage,
  normalizedSourceValues,
  sourceValuesFromSnapshot,
  visibilityFromSnapshot,
  type SourceFormValues,
  type VisibilityValues,
} from "../components/settings/settings-model";

export default function useSettingsController() {
  const widget = useWidgetSettings();
  const [sources, setSources] = useState<SourceFormValues | null>(null);
  const [visible, setVisible] = useState<VisibilityValues>(() =>
    visibilityFromSnapshot(widget.settings),
  );
  const [darkMode, setDarkMode] = useState(widget.settings.darkMode);
  const [theme, setTheme] = useState<ThemeId>(widget.settings.theme);
  const [loadingSources, setLoadingSources] = useState(true);
  const [autoUpdate, setAutoUpdate] = useState(false);
  const [loadingUpdateSettings, setLoadingUpdateSettings] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const persistence = useSettingsPersistence((message) => setError(message));

  useEffect(() => {
    let mounted = true;
    const load = async () => {
      try {
        const snapshot = await getSourceSettings();
        if (mounted) {
          setSources(sourceValuesFromSnapshot(snapshot));
          setError(null);
        }
      } catch (loadError) {
        if (mounted) setError(errorMessage(loadError));
      } finally {
        if (mounted) setLoadingSources(false);
      }
    };
    void load();
    return () => {
      mounted = false;
    };
  }, []);

  useEffect(() => {
    let mounted = true;
    const load = async () => {
      try {
        const snapshot = await getUpdateSettings();
        if (mounted) setAutoUpdate(snapshot.autoUpdate);
      } catch (loadError) {
        if (mounted) setError(errorMessage(loadError));
      } finally {
        if (mounted) setLoadingUpdateSettings(false);
      }
    };
    void load();
    return () => {
      mounted = false;
    };
  }, []);

  useEffect(() => {
    if (!widget.loading) {
      setVisible(visibilityFromSnapshot(widget.settings));
      setDarkMode(widget.settings.darkMode);
      setTheme(widget.settings.theme);
    }
  }, [widget.loading, widget.settings]);

  const sendPreview = (
    nextTheme: ThemeId,
    nextDarkMode: boolean,
    nextVisible: VisibilityValues,
    nextSources: SourceFormValues | null,
  ) => {
    if (!nextSources) return;
    persistence.sendPreview(
      createWidgetSettingsPreview(
        nextTheme,
        nextDarkMode,
        nextVisible,
        nextSources,
      ),
    );
  };

  const saveWidget = (
    nextTheme: ThemeId,
    nextDarkMode: boolean,
    nextVisible: VisibilityValues,
  ) => {
    const snapshot: WidgetSettingsSnapshot = {
      darkMode: nextDarkMode,
      theme: nextTheme,
      visibleProviders: providerOrder.map((provider) => ({
        provider,
        visible: nextVisible[provider],
      })),
    };
    persistence.saveWidget(snapshot);
  };

  const updateSource = (
    provider: ProviderId,
    changes: Partial<SourceSettings>,
  ) => {
    if (!sources) return;
    const nextSources = {
      ...sources,
      [provider]: { ...sources[provider], ...changes },
    };
    setError(null);
    setSources(nextSources);
    if ("enabled" in changes) {
      sendPreview(theme, darkMode, visible, nextSources);
    }
    persistence.saveSource(normalizedSourceValues(nextSources)[provider]);
  };

  const closeSettings = async () => {
    await persistence.flush();
    try {
      await closeCurrentWindow();
    } catch {
      setError("Could not close Settings.");
    }
  };

  const toggleProviderVisibility = (provider: ProviderId, next: boolean) => {
    const nextVisible = { ...visible, [provider]: next };
    setError(null);
    setVisible(nextVisible);
    sendPreview(theme, darkMode, nextVisible, sources);
    saveWidget(theme, darkMode, nextVisible);
  };

  const toggleSource = (provider: ProviderId, enabled: boolean) => {
    updateSource(provider, { enabled });
  };

  const chooseSourceRoot = async (
    provider: ProviderId,
    platform: SourcePlatform,
  ) => {
    setError(null);
    try {
      const snapshot = await pickSourceRoot(provider, platform);
      if (snapshot) setSources(sourceValuesFromSnapshot(snapshot));
    } catch (openError) {
      setError(errorMessage(openError));
    }
  };

  const clearSourceRoot = (provider: ProviderId, platform: SourcePlatform) => {
    updateSource(
      provider,
      platform === "windows" ? { windowsRoot: null } : { wslRoot: null },
    );
  };

  const changeSourceRoot = (
    provider: ProviderId,
    platform: SourcePlatform,
    root: string,
  ) => {
    updateSource(
      provider,
      platform === "windows" ? { windowsRoot: root } : { wslRoot: root },
    );
  };

  const toggleDarkMode = (next: boolean) => {
    setError(null);
    setDarkMode(next);
    sendPreview(theme, next, visible, sources);
    saveWidget(theme, next, visible);
  };

  const toggleTheme = (next: ThemeId) => {
    setError(null);
    setTheme(next);
    sendPreview(next, darkMode, visible, sources);
    saveWidget(next, darkMode, visible);
  };

  const toggleAutoUpdate = (next: boolean) => {
    setError(null);
    setAutoUpdate(next);
    const settings: UpdateSettingsSnapshot = { autoUpdate: next };
    persistence.saveUpdate(settings);
  };

  return {
    closeSettings,
    darkMode,
    autoUpdate,
    loadingUpdateSettings,
    theme,
    error,
    loadingSources,
    onDarkModeToggle: toggleDarkMode,
    onAutoUpdateToggle: toggleAutoUpdate,
    onThemeToggle: toggleTheme,
    onProviderVisibilityToggle: toggleProviderVisibility,
    onSourceRootChoose: chooseSourceRoot,
    onSourceRootChange: changeSourceRoot,
    onSourceRootClear: clearSourceRoot,
    onSourceToggle: toggleSource,
    sources,
    visible,
    widgetError: widget.error ? errorMessage(widget.error) : null,
  };
}
