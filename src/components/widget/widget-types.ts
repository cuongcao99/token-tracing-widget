import type { ProviderUsageSummary, UsageState } from "../../lib/usage-summary";

export interface WidgetProviderRowProps {
  usage: ProviderUsageSummary;
}

export function stateLabel(state: UsageState): string {
  return state.charAt(0).toUpperCase() + state.slice(1);
}

export function headerStateLabel(state: UsageState): string {
  switch (state) {
    case "active":
      return "Live";
    case "loading":
      return "Loading";
    case "idle":
      return "Idle";
    case "unavailable":
      return "Unavailable";
    case "stale":
      return "Stale";
  }
}

export function formatTokens(tokens: number | undefined): string {
  return tokens === undefined ? "Unavailable" : tokens.toLocaleString("en-US");
}
