import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useUsageSummary } from "../../hooks/useUsageSummary";
import { useWidgetSettings } from "../../hooks/useWidgetSettings";
import { createWidgetViewModel } from "../../lib/widget-view-model";
import type { WindowResizeDirection } from "../../lib/window-actions";
import { syncWidgetWindowHeight } from "../../lib/window-sizing";
import styles from "../../styles/widget/surface.module.css";
import WindowGrip from "../shared/WindowGrip";
import WindowResizeHandles from "../shared/WindowResizeHandles";
import WidgetHeader from "./WidgetHeader";
import ProviderUsageRow from "./ProviderUsageRow";
import WidgetTotal from "./WidgetTotal";

function measureWidgetContentHeight(
  root: HTMLElement | null,
  providerList: HTMLElement | null,
): number | undefined {
  if (!root || !providerList || providerList.scrollHeight <= 0) return undefined;

  const lastChild = providerList.lastElementChild;
  if (!lastChild) return undefined;

  const listRect = providerList.getBoundingClientRect();
  const lastChildRect = lastChild.getBoundingClientRect();
  const paddingBottom =
    Number.parseFloat(getComputedStyle(providerList).paddingBottom) || 0;
  const listHeight =
    lastChildRect.bottom - listRect.top + providerList.scrollTop + paddingBottom;

  if (listHeight === undefined || !Number.isFinite(listHeight) || listHeight <= 0) {
    return undefined;
  }

  return root.clientHeight - providerList.clientHeight + listHeight;
}

export default function TokenTracingWidget() {
  const { summary } = useUsageSummary();
  const { settings, previewSourceEnabled } = useWidgetSettings();
  const viewModel = useMemo(
    () =>
      createWidgetViewModel({
        summary,
        settings,
        previewSourceEnabled,
      }),
    [previewSourceEnabled, settings, summary],
  );
  const contentKey = useMemo(
    () =>
      summary.providers
        .map(({ provider, rateLimits, sessions }) =>
          [
            provider,
            rateLimits?.length ?? 0,
            sessions.map(({ id, state }) => `${id}:${state}`).join(","),
          ].join(":"),
        )
        .join("|"),
    [summary],
  );
  const visibilityKey = useMemo(
    () =>
      settings.visibleProviders
        .map(({ provider, visible }) => `${provider}:${visible}`)
        .join("|"),
    [settings.visibleProviders],
  );
  const sourcePreviewKey = useMemo(
    () =>
      previewSourceEnabled === null
        ? "none"
        : Object.entries(previewSourceEnabled)
            .map(([provider, enabled]) => `${provider}:${enabled}`)
            .join("|"),
    [previewSourceEnabled],
  );
  const rootRef = useRef<HTMLElement>(null);
  const providerListRef = useRef<HTMLElement>(null);
  const autoFitHeightRef = useRef(true);
  const previousLayoutRef = useRef({ sourcePreviewKey, contentKey });
  const ignoredContentKeyRef = useRef<string | null>(null);
  const [layoutRevision, setLayoutRevision] = useState(0);
  const onSessionToggle = useCallback(() => {
    setLayoutRevision((revision) => revision + 1);
  }, []);
  const onResizeStart = useCallback((direction: WindowResizeDirection) => {
    if (direction !== "East" && direction !== "West") {
      autoFitHeightRef.current = false;
    }
  }, []);

  useEffect(() => {
    const previous = previousLayoutRef.current;
    if (previous.sourcePreviewKey !== sourcePreviewKey) {
      ignoredContentKeyRef.current = previous.contentKey;
    }
    previousLayoutRef.current = { sourcePreviewKey, contentKey };
  }, [contentKey, sourcePreviewKey]);

  useEffect(() => {
    if (!autoFitHeightRef.current) return;
    if (
      ignoredContentKeyRef.current !== null &&
      ignoredContentKeyRef.current !== contentKey
    ) {
      ignoredContentKeyRef.current = null;
      return;
    }

    const measuredContentHeight = measureWidgetContentHeight(
      rootRef.current,
      providerListRef.current,
    );
    const request =
      measuredContentHeight === undefined
        ? syncWidgetWindowHeight(viewModel.visibleProviderCount, undefined, true)
        : syncWidgetWindowHeight(
            viewModel.visibleProviderCount,
            measuredContentHeight,
            true,
          );
    void request.catch(() => undefined);
  }, [contentKey, layoutRevision, visibilityKey]);

  return (
    <main
      ref={rootRef}
      className={styles.root}
      data-theme={viewModel.theme}
      data-color-mode={viewModel.colorMode}
      aria-label="Token usage summary"
    >
      <WindowGrip windowName="widget" />
      <WidgetHeader activityState={summary.state} />
      <section
        ref={providerListRef}
        className={styles.providerList}
        aria-label="Provider usage"
      >
        {viewModel.providers.map((provider) => (
          <ProviderUsageRow
            key={provider.provider}
            usage={provider}
            onSessionToggle={onSessionToggle}
          />
        ))}
      </section>
      {viewModel.visibleProviderCount > 0 ? (
        <WidgetTotal tokens={viewModel.totalTokens} />
      ) : null}
      <WindowResizeHandles
        windowName="widget"
        onResizeStart={onResizeStart}
      />
    </main>
  );
}
