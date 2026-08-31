import {
  useEffect,
  useRef,
  useState,
  type FormEvent,
  type MouseEvent,
} from "react";
import { useUsageSummary } from "./useUsageSummary";
import { useWidgetSettings } from "./useWidgetSettings";
import {
  closeCurrentWindow,
  startCurrentWindowDrag,
} from "../lib/window-actions";
import { providerOrder, type ProviderId } from "../lib/provider";
import {
  getSourceSettings,
  updateSourceSettings,
  type SourceSettings,
} from "../lib/source-settings";
import { formatRelativeUpdate } from "../lib/usage-summary";
import { updateWidgetSettings } from "../lib/widget-settings";
import { emitWidgetSettingsPreview } from "../lib/widget-settings-preview";
import {
  createWidgetSettingsPreview,
  errorMessage,
  normalizedSourceValues,
  sourceValuesFromSnapshot,
  visibilityFromSnapshot,
  type SourceFormValues,
  type VisibilityValues,
} from "../components/settings/settings-model";

type ExpandedValues = Record<ProviderId, boolean>;

export default function useSettingsController() {
  const { summary } = useUsageSummary();
  const widget = useWidgetSettings();
  const [sources, setSources] = useState<SourceFormValues | null>(null);
  const [visible, setVisible] = useState<VisibilityValues>(() =>
    visibilityFromSnapshot(widget.settings),
  );
  const [darkMode, setDarkMode] = useState(widget.settings.darkMode);
  const [savedWidgetSettings, setSavedWidgetSettings] = useState(
    widget.persistedSettings,
  );
  const [savedSources, setSavedSources] = useState<SourceFormValues | null>(null);
  const [expanded, setExpanded] = useState<ExpandedValues>({
    claude: false,
    codex: false,
  });
  const [loadingSources, setLoadingSources] = useState(true);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [saved, setSaved] = useState(false);
  const pendingPreview = useRef<Promise<void>>(Promise.resolve());

  useEffect(() => {
    let mounted = true;
    const load = async () => {
      try {
        const snapshot = await getSourceSettings();
        if (mounted) {
          const nextSources = sourceValuesFromSnapshot(snapshot);
          setSources(nextSources);
          setSavedSources(nextSources);
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
    if (!widget.loading) {
      setVisible(visibilityFromSnapshot(widget.settings));
      setDarkMode(widget.settings.darkMode);
      setSavedWidgetSettings(widget.persistedSettings);
    }
  }, [widget.loading, widget.persistedSettings, widget.settings]);

  const sendPreview = (
    nextDarkMode: boolean,
    nextVisible: VisibilityValues,
    nextSources: SourceFormValues | null,
  ) => {
    if (!nextSources) return;

    let request: Promise<void>;
    try {
      request = emitWidgetSettingsPreview(
        createWidgetSettingsPreview(nextDarkMode, nextVisible, nextSources),
      );
    } catch {
      setError("Could not preview settings.");
      return;
    }

    const pending = Promise.allSettled([pendingPreview.current, request]).then(
      (results) => {
        if (results.some((result) => result.status === "rejected")) {
          throw new Error("preview_failed");
        }
      },
    );
    pendingPreview.current = pending.catch(() => undefined);
    void pending.catch(() => setError("Could not preview settings."));
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
    setSaved(false);
    setSources(nextSources);
    if ("enabled" in changes) {
      sendPreview(darkMode, visible, nextSources);
    }
  };

  const save = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    if (!sources) return;

    setSaving(true);
    setSaved(false);
    setError(null);
    try {
      const nextSources = normalizedSourceValues(sources);
      await pendingPreview.current;
      for (const provider of providerOrder) {
        const source = nextSources[provider];
        await updateSourceSettings({
          provider,
          enabled: source.enabled,
          rootOverride: source.rootOverride?.trim() || null,
        });
      }
      const nextWidgetSettings = await updateWidgetSettings({
        darkMode,
        visibleProviders: providerOrder.map((provider) => ({
          provider,
          visible: visible[provider],
        })),
      });
      setSources(nextSources);
      setSavedSources(nextSources);
      setSavedWidgetSettings(nextWidgetSettings);
      setSaved(true);
    } catch (saveError) {
      setError(errorMessage(saveError));
    } finally {
      setSaving(false);
    }
  };

  const providerStatuses = providerOrder.map((provider) => {
    const usage = summary.providers.find((entry) => entry.provider === provider);
    return {
      provider,
      state: usage?.state ?? "unavailable",
      updated: usage?.lastUpdatedAt
        ? formatRelativeUpdate(usage.lastUpdatedAt)
        : "No updates yet",
    };
  });

  const handleWindowMouseDown = (event: MouseEvent<HTMLElement>) => {
    if (event.button !== 0) return;
    if (event.target instanceof Element && event.target.closest("button")) return;
    void startCurrentWindowDrag().catch(() => undefined);
  };

  const closeSettings = async () => {
    try {
      await pendingPreview.current;
      const baselineSources = savedSources ?? sources;
      if (baselineSources) {
        await emitWidgetSettingsPreview(
          createWidgetSettingsPreview(
            savedWidgetSettings.darkMode,
            visibilityFromSnapshot(savedWidgetSettings),
            baselineSources,
          ),
        );
      }
    } catch {
      // A preview transport failure must not make the close control unusable.
    }

    try {
      await closeCurrentWindow();
    } catch {
      setError("Could not close Settings.");
    }
  };

  const toggleProviderVisibility = (provider: ProviderId, next: boolean) => {
    const nextVisible = { ...visible, [provider]: next };
    setSaved(false);
    setVisible(nextVisible);
    sendPreview(darkMode, nextVisible, sources);
  };

  const toggleSource = (provider: ProviderId, enabled: boolean) => {
    updateSource(provider, { enabled });
  };

  const updateSourceRoot = (provider: ProviderId, rootOverride: string) => {
    updateSource(provider, { rootOverride });
  };

  const toggleSourceRoot = (provider: ProviderId) => {
    setExpanded((current) => ({
      ...current,
      [provider]: !current[provider],
    }));
  };

  const toggleDarkMode = (next: boolean) => {
    setSaved(false);
    setDarkMode(next);
    sendPreview(next, visible, sources);
  };

  return {
    closeSettings,
    darkMode,
    error,
    expanded,
    handleWindowMouseDown,
    loadingSources,
    onDarkModeToggle: toggleDarkMode,
    onProviderVisibilityToggle: toggleProviderVisibility,
    onSourceRootChange: updateSourceRoot,
    onSourceRootToggle: toggleSourceRoot,
    onSourceToggle: toggleSource,
    providerStatuses,
    save,
    saved,
    saving,
    sources,
    summary,
    visible,
    widgetError: widget.error ? errorMessage(widget.error) : null,
  };
}
