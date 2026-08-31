import { formatRelativeUpdate } from "../../lib/usage-summary";
import { providerMeta } from "../../lib/provider";
import ProviderDot from "../shared/ProviderDot";
import type { WidgetProviderRowProps } from "./widget-types";
import { formatTokens, stateLabel } from "./widget-types";

export default function ProviderUsageRow({ usage }: WidgetProviderRowProps) {
  const meta = providerMeta[usage.provider];
  const session = formatTokens(usage.currentSessionTokens);
  const today = formatTokens(usage.todayTokens);

  return (
    <article className={`widget-provider widget-provider--${usage.provider}`}>
      <div className="widget-provider__heading">
        <h2>
          <ProviderDot provider={usage.provider} />
          <span>{meta.name}</span>
        </h2>
        <span className={`provider-status provider-status--${usage.state}`}>
          <span className="provider-status__dot" aria-hidden="true" />
          {stateLabel(usage.state)}
        </span>
      </div>
      <div className="widget-provider__metrics">
        <div className="widget-metric">
          <span>Session</span>
          <strong aria-label={`Session: ${session} tokens`}>{session}</strong>
        </div>
        <div className="widget-metric">
          <span>Today</span>
          <strong aria-label={`Today: ${today} tokens`}>{today}</strong>
        </div>
        <span className="widget-provider__updated">
          {formatRelativeUpdate(usage.lastUpdatedAt)}
        </span>
      </div>
    </article>
  );
}
