import { memo } from "react";
import ProviderSection from "./ProviderSection";
import UsageMetrics from "./UsageMetrics";
import type { WidgetProviderRowProps } from "./widget-types";
import { formatTokens } from "./widget-types";

export function areProviderUsageRowsEqual(
  previous: WidgetProviderRowProps,
  next: WidgetProviderRowProps,
): boolean {
  const before = previous.usage;
  const after = next.usage;
  return (
    before.provider === after.provider &&
    before.identity.name === after.identity.name &&
    before.identity.displayName === after.identity.displayName &&
    before.identity.logoSrc === after.identity.logoSrc &&
    before.identity.logoVariant === after.identity.logoVariant &&
    before.identity.fontRole === after.identity.fontRole &&
    before.identity.accent === after.identity.accent &&
    before.status.state === after.status.state &&
    before.status.label === after.status.label &&
    before.metrics.sessionTokens === after.metrics.sessionTokens &&
    before.metrics.todayTokens === after.metrics.todayTokens &&
    before.metrics.updatedLabel === after.metrics.updatedLabel
  );
}

export const ProviderUsageRow = memo(function ProviderUsageRow({
  usage,
}: WidgetProviderRowProps) {
  const session = formatTokens(usage.metrics.sessionTokens);
  const today = formatTokens(usage.metrics.todayTokens);

  return (
    <ProviderSection
      identity={usage.identity}
      status={usage.status}
    >
      <UsageMetrics
        metrics={[
          {
            label: "Session",
            value: session,
            ariaLabel: `Session: ${session} tokens`,
          },
          {
            label: "Today",
            value: today,
            ariaLabel: `Today: ${today} tokens`,
          },
        ]}
        updatedLabel={usage.metrics.updatedLabel}
      />
    </ProviderSection>
  );
}, areProviderUsageRowsEqual);

export default ProviderUsageRow;
