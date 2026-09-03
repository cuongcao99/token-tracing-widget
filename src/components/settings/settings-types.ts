import type { ProviderId } from "../../lib/provider";
import type { SourceHealth } from "../../lib/usage-summary";

export interface ProviderStatusView {
  provider: ProviderId;
  state: string;
  updated: string;
}

export type SourceDisplayState = "available" | "unavailable" | "off";

export function sourceHealthState(
  provider: ProviderId,
  health: SourceHealth[],
  enabled: boolean,
): SourceDisplayState {
  if (!enabled) return "off";
  const state = health.find((entry) => entry.provider === provider)?.state;
  switch (state) {
    case "detected":
    case "limited":
    case "malformed":
      return "available";
    case "disabled":
      return "off";
    default:
      return "unavailable";
  }
}

export function sourceHealthLabel(
  provider: ProviderId,
  health: SourceHealth[],
  enabled: boolean,
): string {
  switch (sourceHealthState(provider, health, enabled)) {
    case "available":
      return "Available";
    case "off":
      return "Off";
    case "unavailable":
      return "Unavailable";
  }
}

export function providerActivityLabel(state: string): string {
  switch (state) {
    case "active":
      return "Active";
    case "idle":
      return "Idle";
    case "loading":
      return "Checking";
    default:
      return "Unavailable";
  }
}
