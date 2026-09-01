import { useEffect, useMemo } from "react";
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

  useEffect(() => {
    void syncWidgetWindowHeight(viewModel.visibleProviderCount).catch(() => undefined);
  }, [viewModel.visibleProviderCount]);

  return (
    <main
      className={styles.root}
      data-theme={viewModel.theme}
      data-color-mode={viewModel.colorMode}
      aria-label="Token usage summary"
    >
      <WindowGrip windowName="widget" />
      <WidgetHeader activityState={summary.state} />
      <section className={styles.providerList} aria-label="Provider usage">
        {viewModel.providers.map((provider) => (
          <ProviderUsageRow key={provider.provider} usage={provider} />
        ))}
      </section>
      <WidgetTotal tokens={viewModel.totalTokens} />
      <WindowResizeHandles windowName="widget" />
    </main>
  );
}
