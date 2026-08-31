import { useEffect, useMemo } from "react";
import { useUsageSummary } from "../../hooks/useUsageSummary";
import { useWidgetSettings } from "../../hooks/useWidgetSettings";
import { providerOrder } from "../../lib/provider";
import { syncWidgetWindowHeight } from "../../lib/window-sizing";
import WindowResizeHandles from "../shared/WindowResizeHandles";
import WidgetHeader from "./WidgetHeader";
import ProviderUsageRow from "./ProviderUsageRow";
import WidgetTotal from "./WidgetTotal";

export default function TokenTracingWidget() {
  const { summary } = useUsageSummary();
  const { settings, previewSourceEnabled } = useWidgetSettings();
  const visibleProviders = useMemo(
    () => new Set(settings.visibleProviders.filter((entry) => entry.visible).map((entry) => entry.provider)),
    [settings.visibleProviders],
  );
  const visibleProviderCount = visibleProviders.size;

  useEffect(() => {
    void syncWidgetWindowHeight(visibleProviderCount).catch(() => undefined);
  }, [visibleProviderCount]);

  const totalTokens = useMemo(() => {
    if (!previewSourceEnabled) return summary.todayTokens;
    return summary.providers.reduce(
      (total, usage) =>
        previewSourceEnabled[usage.provider] === false
          ? total
          : total + usage.todayTokens,
      0,
    );
  }, [previewSourceEnabled, summary.providers, summary.todayTokens]);

  return (
    <main
      className={`widget widget--${settings.darkMode ? "dark" : "light"}`}
      aria-label="Token usage summary"
    >
      <WidgetHeader state={summary.state} />
      <section className="widget-provider-list" aria-label="Provider usage">
        {providerOrder.map((provider) => {
          const usage = summary.providers.find((entry) => entry.provider === provider);
          if (!usage || !visibleProviders.has(provider)) return null;
          const displayUsage =
            previewSourceEnabled?.[provider] === false
              ? { ...usage, state: "unavailable" as const }
              : usage;
          return <ProviderUsageRow key={provider} usage={displayUsage} />;
        })}
      </section>
      <WidgetTotal tokens={totalTokens} />
      <WindowResizeHandles windowName="widget" />
    </main>
  );
}
