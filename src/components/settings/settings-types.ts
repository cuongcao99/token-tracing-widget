import type { ProviderId } from "../../lib/provider";
import type { SourceHealth } from "../../lib/usage-summary";

export interface ProviderStatusView {
  provider: ProviderId;
  state: string;
  updated: string;
}

export function sourceHealthLabel(
  provider: ProviderId,
  health: SourceHealth[],
  enabled: boolean,
): string {
  if (!enabled) return "Off";
  const state = health.find((entry) => entry.provider === provider)?.state;
  switch (state) {
    case "detected":
      return "Ready";
    case "limited":
      return "Limited";
    case "malformed":
      return "Check source";
    case "permission_denied":
      return "Needs access";
    case "invalid_root":
    case "not_detected":
    case "unavailable":
      return "Unavailable";
    case "disabled":
      return "Off";
    default:
      return "Checking";
  }
}
