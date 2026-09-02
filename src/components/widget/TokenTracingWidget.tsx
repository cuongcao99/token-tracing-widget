import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useUsageSummary } from "../../hooks/useUsageSummary";
import { useWidgetSettings } from "../../hooks/useWidgetSettings";
import { createWidgetViewModel } from "../../lib/widget-view-model";
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
  return root.clientHeight - providerList.clientHeight + providerList.scrollHeight;
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
  const rootRef = useRef<HTMLElement>(null);
  const providerListRef = useRef<HTMLElement>(null);
  const [layoutRevision, setLayoutRevision] = useState(0);
  const onSessionToggle = useCallback(() => {
    setLayoutRevision((revision) => revision + 1);
  }, []);

  useEffect(() => {
    const measuredContentHeight = measureWidgetContentHeight(
      rootRef.current,
      providerListRef.current,
    );
    const request =
      measuredContentHeight === undefined
        ? syncWidgetWindowHeight(viewModel.visibleProviderCount)
        : syncWidgetWindowHeight(
            viewModel.visibleProviderCount,
            measuredContentHeight,
          );
    void request.catch(() => undefined);
  }, [layoutRevision, viewModel]);

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
      <WidgetTotal tokens={viewModel.totalTokens} />
      <WindowResizeHandles windowName="widget" />
    </main>
  );
}
