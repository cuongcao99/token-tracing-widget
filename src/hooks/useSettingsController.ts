import { useEffect, useRef, useState } from "react";
import { useUsageSummary } from "./useUsageSummary";
import { useWidgetSettings } from "./useWidgetSettings";
import { closeCurrentWindow } from "../lib/window-actions";
import { providerOrder, type ProviderId } from "../lib/provider";
import {
  getSourceSettings,
  updateSourceSettings,
  type SourceSettings,
} from "../lib/source-settings";
import { formatRelativeUpdate } from "../lib/usage-summary";
import {
  updateWidgetSettings,
  type WidgetSettingsSnapshot,
} from "../lib/widget-settings";
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

const SOURCE_ROOT_SAVE_DEBOUNCE_MS = 350;

type ExpandedValues = Record<ProviderId, boolean>;
type SourceRootTimer = ReturnType<typeof setTimeout>;

export default function useSettingsController() {
  const { summary } = useUsageSummary();
  const widget = useWidgetSettings();
  const [sources, setSources] = useState<SourceFormValues | null>(null);
  const [visible, setVisible] = useState<VisibilityValues>(() =>
    visibilityFromSnapshot(widget.settings),
  );
  const [darkMode, setDarkMode] = useState(widget.settings.darkMode);
  const [expanded, setExpanded] = useState<ExpandedValues>({
    claude: false,
    codex: false,
  });
  const [loadingSources, setLoadingSources] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const pendingPreview = useRef<Promise<void>>(Promise.resolve());
  const pendingPersistence = useRef<Promise<void>>(Promise.resolve());
  const sourceRootTimers = useRef(new Map<ProviderId, SourceRootTimer>());
  const pendingSourceRootSnapshots = useRef(
    new Map<ProviderId, SourceFormValues>(),
  );

  useEffect(() => {
    let mounted = true;
    const load = async () => {
      try {
        const snapshot = await getSourceSettings();
        if (mounted) {
          const nextSources = sourceValuesFromSnapshot(snapshot);
          setSources(nextSources);
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
    }
  }, [widget.loading, widget.settings]);

  useEffect(
    () => () => {
      for (const timer of sourceRootTimers.current.values()) {
        clearTimeout(timer);
      }
      sourceRootTimers.current.clear();
      pendingSourceRootSnapshots.current.clear();
    },
    [],
  );

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

  const enqueuePersistence = (operation: () => Promise<unknown>) => {
    const request = pendingPersistence.current
      .catch(() => undefined)
      .then(operation)
      .then(() => undefined);
    pendingPersistence.current = request.catch(() => undefined);
    void request.catch((persistError) => setError(errorMessage(persistError)));
    return request;
  };

  const enqueueWidgetPersistence = (
    nextDarkMode: boolean,
    nextVisible: VisibilityValues,
  ) => {
    const snapshot: WidgetSettingsSnapshot = {
      darkMode: nextDarkMode,
      visibleProviders: providerOrder.map((provider) => ({
        provider,
        visible: nextVisible[provider],
      })),
    };
    void enqueuePersistence(() => updateWidgetSettings(snapshot));
  };

  const enqueueSourcePersistence = (
    provider: ProviderId,
    nextSources: SourceFormValues,
  ) => {
    const source = normalizedSourceValues(nextSources)[provider];
    void enqueuePersistence(() =>
      updateSourceSettings({
        provider,
        enabled: source.enabled,
        rootOverride: source.rootOverride,
      }),
    );
  };

  const cancelSourceRootPersistence = (provider: ProviderId) => {
    const timer = sourceRootTimers.current.get(provider);
    if (timer !== undefined) clearTimeout(timer);
    sourceRootTimers.current.delete(provider);
    pendingSourceRootSnapshots.current.delete(provider);
  };

  const flushSourceRootPersistence = (provider: ProviderId) => {
    const timer = sourceRootTimers.current.get(provider);
    if (timer !== undefined) clearTimeout(timer);
    sourceRootTimers.current.delete(provider);

    const snapshot = pendingSourceRootSnapshots.current.get(provider);
    pendingSourceRootSnapshots.current.delete(provider);
    if (snapshot) enqueueSourcePersistence(provider, snapshot);
  };

  const scheduleSourceRootPersistence = (
    provider: ProviderId,
    nextSources: SourceFormValues,
  ) => {
    cancelSourceRootPersistence(provider);
    pendingSourceRootSnapshots.current.set(provider, nextSources);
    const timer = setTimeout(() => {
      sourceRootTimers.current.delete(provider);
      const snapshot = pendingSourceRootSnapshots.current.get(provider);
      pendingSourceRootSnapshots.current.delete(provider);
      if (snapshot) enqueueSourcePersistence(provider, snapshot);
    }, SOURCE_ROOT_SAVE_DEBOUNCE_MS);
    sourceRootTimers.current.set(provider, timer);
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
      cancelSourceRootPersistence(provider);
      sendPreview(darkMode, visible, nextSources);
      enqueueSourcePersistence(provider, nextSources);
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

  const closeSettings = async () => {
    for (const provider of providerOrder) {
      flushSourceRootPersistence(provider);
    }

    await pendingPreview.current;
    await pendingPersistence.current;

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
    sendPreview(darkMode, nextVisible, sources);
    enqueueWidgetPersistence(darkMode, nextVisible);
  };

  const toggleSource = (provider: ProviderId, enabled: boolean) => {
    updateSource(provider, { enabled });
  };

  const updateSourceRoot = (provider: ProviderId, rootOverride: string) => {
    if (!sources) return;
    const nextSources = {
      ...sources,
      [provider]: { ...sources[provider], rootOverride },
    };
    setError(null);
    setSources(nextSources);
    scheduleSourceRootPersistence(provider, nextSources);
  };

  const toggleSourceRoot = (provider: ProviderId) => {
    setExpanded((current) => ({
      ...current,
      [provider]: !current[provider],
    }));
  };

  const toggleDarkMode = (next: boolean) => {
    setError(null);
    setDarkMode(next);
    sendPreview(next, visible, sources);
    enqueueWidgetPersistence(next, visible);
  };

  return {
    closeSettings,
    darkMode,
    error,
    expanded,
    loadingSources,
    onDarkModeToggle: toggleDarkMode,
    onProviderVisibilityToggle: toggleProviderVisibility,
    onSourceRootBlur: flushSourceRootPersistence,
    onSourceRootChange: updateSourceRoot,
    onSourceRootToggle: toggleSourceRoot,
    onSourceToggle: toggleSource,
    providerStatuses,
    sources,
    summary,
    visible,
    widgetError: widget.error ? errorMessage(widget.error) : null,
  };
}
