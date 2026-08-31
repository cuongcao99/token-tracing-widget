import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { isProviderId, providerOrder, type ProviderId } from "./provider";

export type UsageState = "loading" | "active" | "idle" | "unavailable" | "stale";

export interface SourceHealth {
  provider: ProviderId;
  state: string;
}

export interface ProviderUsageSummary {
  provider: ProviderId;
  state: UsageState;
  currentSessionTokens?: number;
  todayTokens: number;
  lastUpdatedAt?: string;
}

export interface UsageSummary {
  state: UsageState;
  provider?: string;
  currentSessionTokens?: number;
  todayTokens: number;
  lastUpdatedAt?: string;
  sourceHealth: SourceHealth[];
  providers: ProviderUsageSummary[];
}

export const USAGE_SUMMARY_CHANGED_EVENT = "usage-summary-changed";

const summaryKeys = [
  "state",
  "provider",
  "currentSessionTokens",
  "todayTokens",
  "lastUpdatedAt",
  "sourceHealth",
  "providers",
] as const;

const sourceHealthKeys = ["provider", "state"] as const;
const providerSummaryKeys = [
  "provider",
  "state",
  "currentSessionTokens",
  "todayTokens",
  "lastUpdatedAt",
] as const;
const states = new Set<UsageState>([
  "loading",
  "active",
  "idle",
  "unavailable",
  "stale",
]);

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function hasOnlyKeys(
  value: Record<string, unknown>,
  allowed: readonly string[],
): boolean {
  return Object.keys(value).every((key) =>
    allowed.some((name) => name === key),
  );
}

function isTokenCount(value: unknown): value is number {
  return (
    typeof value === "number" &&
    Number.isSafeInteger(value) &&
    value >= 0
  );
}

export function parseUsageSummary(value: unknown): UsageSummary | null {
  if (!isRecord(value) || !hasOnlyKeys(value, summaryKeys)) return null;
  if (typeof value.state !== "string" || !states.has(value.state as UsageState)) {
    return null;
  }
  const todayTokens = value.todayTokens;
  if (!isTokenCount(todayTokens)) return null;
  if ("provider" in value && typeof value.provider !== "string") return null;
  if (
    "currentSessionTokens" in value &&
    !isTokenCount(value.currentSessionTokens)
  ) {
    return null;
  }
  if (
    "lastUpdatedAt" in value &&
    (typeof value.lastUpdatedAt !== "string" ||
      Number.isNaN(Date.parse(value.lastUpdatedAt)))
  ) {
    return null;
  }
  if (!Array.isArray(value.sourceHealth)) return null;

  const sourceHealth: SourceHealth[] = [];
  for (const entry of value.sourceHealth) {
    if (
      !isRecord(entry) ||
      !hasOnlyKeys(entry, sourceHealthKeys) ||
      !isProviderId(entry.provider) ||
      typeof entry.state !== "string"
    ) {
      return null;
    }
    sourceHealth.push({ provider: entry.provider, state: entry.state });
  }

  if (
    !Array.isArray(value.providers) ||
    value.providers.length !== providerOrder.length
  ) {
    return null;
  }

  const seenProviders = new Set<ProviderId>();
  const providers: ProviderUsageSummary[] = [];
  for (const entry of value.providers) {
    if (
      !isRecord(entry) ||
      !hasOnlyKeys(entry, providerSummaryKeys) ||
      !isProviderId(entry.provider) ||
      seenProviders.has(entry.provider) ||
      typeof entry.state !== "string" ||
      !states.has(entry.state as UsageState) ||
      !isTokenCount(entry.todayTokens)
    ) {
      return null;
    }
    if (
      "currentSessionTokens" in entry &&
      !isTokenCount(entry.currentSessionTokens)
    ) {
      return null;
    }
    if (
      "lastUpdatedAt" in entry &&
      (typeof entry.lastUpdatedAt !== "string" ||
        Number.isNaN(Date.parse(entry.lastUpdatedAt)))
    ) {
      return null;
    }

    seenProviders.add(entry.provider);
    providers.push({
      provider: entry.provider,
      state: entry.state as UsageState,
      ...(isTokenCount(entry.currentSessionTokens)
        ? { currentSessionTokens: entry.currentSessionTokens }
        : {}),
      todayTokens: entry.todayTokens,
      ...(typeof entry.lastUpdatedAt === "string"
        ? { lastUpdatedAt: entry.lastUpdatedAt }
        : {}),
    });
  }

  if (seenProviders.size !== providerOrder.length) return null;

  return {
    state: value.state as UsageState,
    ...(typeof value.provider === "string" ? { provider: value.provider } : {}),
    ...(isTokenCount(value.currentSessionTokens)
      ? { currentSessionTokens: value.currentSessionTokens }
      : {}),
    todayTokens,
    ...(typeof value.lastUpdatedAt === "string"
      ? { lastUpdatedAt: value.lastUpdatedAt }
      : {}),
    sourceHealth,
    providers,
  };
}

export async function getUsageSummary(): Promise<UsageSummary> {
  const value = await invoke<unknown>("get_usage_summary");
  const summary = parseUsageSummary(value);
  if (!summary) {
    throw new Error("invalid_usage_summary");
  }
  return summary;
}

export function listenForUsageSummary(
  onSummary: (summary: UsageSummary) => void,
): Promise<UnlistenFn> {
  return listen<unknown>(USAGE_SUMMARY_CHANGED_EVENT, (event) => {
    const summary = parseUsageSummary(event.payload);
    if (summary) onSummary(summary);
  });
}

export function formatRelativeUpdate(
  lastUpdatedAt?: string,
  nowMs = Date.now(),
): string {
  if (!lastUpdatedAt) return "No updates yet";
  const timestampMs = Date.parse(lastUpdatedAt);
  if (Number.isNaN(timestampMs)) return "No updates yet";
  const elapsedMs = Math.max(0, nowMs - timestampMs);
  if (elapsedMs < 60_000) return "just now";
  const minutes = Math.floor(elapsedMs / 60_000);
  if (minutes < 60) return minutes + " min ago";
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return hours + " hr ago";
  return Math.floor(hours / 24) + " d ago";
}
