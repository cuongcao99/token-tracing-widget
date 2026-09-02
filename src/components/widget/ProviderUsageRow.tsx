import { memo } from "react";
import ProviderSection from "./ProviderSection";
import SessionUsageList from "./SessionUsageList";
import UsageLimits from "./UsageLimits";
import UsageMetrics from "./UsageMetrics";
import type { WidgetProviderRowProps } from "./widget-types";
import { formatTokens } from "./widget-types";
import styles from "../../styles/widget/provider.module.css";

export function areProviderUsageRowsEqual(
  previous: WidgetProviderRowProps,
  next: WidgetProviderRowProps,
): boolean {
  const before = previous.usage;
  const after = next.usage;
  if (
    previous.onSessionToggle !== next.onSessionToggle ||
    before.sessionCount !== after.sessionCount ||
    before.sessions.length !== after.sessions.length ||
    before.rateLimits.length !== after.rateLimits.length
  ) {
    return false;
  }

  for (let index = 0; index < before.sessions.length; index += 1) {
    const beforeSession = before.sessions[index];
    const afterSession = after.sessions[index];
    if (
      beforeSession.id !== afterSession.id ||
      beforeSession.label !== afterSession.label ||
      beforeSession.state !== afterSession.state ||
      beforeSession.todayTokens !== afterSession.todayTokens
    ) {
      return false;
    }
  }

  for (let index = 0; index < before.rateLimits.length; index += 1) {
    const beforeLimit = before.rateLimits[index];
    const afterLimit = after.rateLimits[index];
    if (
      beforeLimit.windowMinutes !== afterLimit.windowMinutes ||
      beforeLimit.usedPercent !== afterLimit.usedPercent ||
      beforeLimit.resetsAt !== afterLimit.resetsAt
    ) {
      return false;
    }
  }

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
  onSessionToggle,
}: WidgetProviderRowProps) {
  const sessionCount = usage.sessionCount.toLocaleString("en-US");
  const today = formatTokens(usage.metrics.todayTokens);
  const sessionCountLabel = `${sessionCount} ${usage.sessionCount === 1 ? "session" : "sessions"} today`;

  return (
    <ProviderSection
      identity={usage.identity}
      status={usage.status}
    >
      <UsageLimits limits={usage.rateLimits} />
      <UsageMetrics
        metrics={[
          {
            label: "Session",
            value: sessionCount,
            ariaLabel: `Session count: ${sessionCountLabel}`,
          },
          {
            label: "Today",
            value: today,
            ariaLabel: `Today: ${today} tokens`,
          },
        ]}
        updatedLabel={usage.metrics.updatedLabel}
      />
      {usage.sessionCount === 0 ? (
        <p className={styles.emptyState}>No activity yet today</p>
      ) : (
        <SessionUsageList
          sessions={usage.sessions}
          onToggle={onSessionToggle}
        />
      )}
    </ProviderSection>
  );
}, areProviderUsageRowsEqual);

export default ProviderUsageRow;
