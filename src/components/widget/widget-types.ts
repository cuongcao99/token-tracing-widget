import type { ProviderUsageSummary, UsageState } from "../../lib/usage-summary";

export interface WidgetProviderRowProps {
  usage: ProviderUsageSummary;
}

export function stateLabel(state: UsageState): string {
  return state.charAt(0).toUpperCase() + state.slice(1);
}

export function formatTokens(tokens: number | undefined): string {
  return tokens === undefined ? "Unavailable" : tokens.toLocaleString("en-US");
}
