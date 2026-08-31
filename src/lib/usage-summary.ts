import {
  listenForUsageSummaryEvent,
  USAGE_SUMMARY_CHANGED_EVENT,
  type UnlistenFn,
} from "./desktop/events";
import { invokeUsageSummary } from "./desktop/commands";
import {
  formatRelativeUpdate,
  parseUsageSummary,
  type ProviderUsageSummary,
  type SourceHealth,
  type UsageState,
  type UsageSummary,
} from "./contracts/usage-summary";

export {
  formatRelativeUpdate,
  parseUsageSummary,
  type ProviderUsageSummary,
  type SourceHealth,
  type UsageState,
  type UsageSummary,
} from "./contracts/usage-summary";
export { USAGE_SUMMARY_CHANGED_EVENT } from "./desktop/events";
export type { UnlistenFn } from "./desktop/events";

export async function getUsageSummary(): Promise<UsageSummary> {
  const value = await invokeUsageSummary();
  const summary = parseUsageSummary(value);
  if (!summary) {
    throw new Error("invalid_usage_summary");
  }
  return summary;
}

export function listenForUsageSummary(
  onSummary: (summary: UsageSummary) => void,
): Promise<UnlistenFn> {
  return listenForUsageSummaryEvent((payload) => {
    const summary = parseUsageSummary(payload);
    if (summary) onSummary(summary);
  });
}
