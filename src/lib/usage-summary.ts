import { invoke } from "@tauri-apps/api/core";

export type UsageState = "loading" | "active" | "idle" | "unavailable" | "stale";

export interface SourceHealth {
  provider: string;
  state: string;
}

export interface UsageSummary {
  state: UsageState;
  provider?: string;
  currentSessionTokens?: number;
  todayTokens: number;
  lastUpdatedAt?: string;
  sourceHealth: SourceHealth[];
}

export function getUsageSummary(): Promise<UsageSummary> {
  return invoke<UsageSummary>("get_usage_summary");
}
