import { isProviderId, providerOrder, type ProviderId } from "../provider";
import {
  hasOnlyKeys,
  isRecord,
  isSafeSessionLabel,
  isSafeTokenCount,
  isValidDateString,
} from "./validation";

export type UsageState = "loading" | "active" | "idle" | "unavailable" | "stale";
export type SessionUsageState = "active" | "idle";

export interface SessionUsageSummary {
  id: string;
  name?: string;
  state: SessionUsageState;
  todayTokens: number;
}

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
  sessions: SessionUsageSummary[];
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
  "sessions",
] as const;
const sessionSummaryKeys = ["id", "name", "state", "todayTokens"] as const;
const usageStates = new Set<UsageState>([
  "loading",
  "active",
  "idle",
  "unavailable",
  "stale",
]);
const sessionUsageStates = new Set<SessionUsageState>(["active", "idle"]);

function isUsageState(value: unknown): value is UsageState {
  return typeof value === "string" && usageStates.has(value as UsageState);
}

function isSessionUsageState(value: unknown): value is SessionUsageState {
  return (
    typeof value === "string" &&
    sessionUsageStates.has(value as SessionUsageState)
  );
}

function parseSessionSummaries(value: unknown): SessionUsageSummary[] | null {
  if (!Array.isArray(value)) return null;

  const seenIds = new Set<string>();
  const sessions: SessionUsageSummary[] = [];
  for (const entry of value) {
    if (
      !isRecord(entry) ||
      !hasOnlyKeys(entry, sessionSummaryKeys) ||
      !isSafeSessionLabel(entry.id) ||
      seenIds.has(entry.id) ||
      !isSessionUsageState(entry.state) ||
      !isSafeTokenCount(entry.todayTokens)
    ) {
      return null;
    }

    if ("name" in entry && !isSafeSessionLabel(entry.name)) return null;

    seenIds.add(entry.id);
    sessions.push({
      id: entry.id,
      ...(typeof entry.name === "string" ? { name: entry.name } : {}),
      state: entry.state,
      todayTokens: entry.todayTokens,
    });
  }

  return sessions;
}

function optionalTokenFields(value: Record<string, unknown>): {
  currentSessionTokens?: number;
} | null {
  if (
    "currentSessionTokens" in value &&
    !isSafeTokenCount(value.currentSessionTokens)
  ) {
    return null;
  }
  return isSafeTokenCount(value.currentSessionTokens)
    ? { currentSessionTokens: value.currentSessionTokens }
    : {};
}

function optionalDateFields(value: Record<string, unknown>): {
  lastUpdatedAt?: string;
} | null {
  if ("lastUpdatedAt" in value && !isValidDateString(value.lastUpdatedAt)) {
    return null;
  }
  return typeof value.lastUpdatedAt === "string"
    ? { lastUpdatedAt: value.lastUpdatedAt }
    : {};
}

export function parseUsageSummary(value: unknown): UsageSummary | null {
  if (!isRecord(value) || !hasOnlyKeys(value, summaryKeys)) return null;
  if (!isUsageState(value.state) || !isSafeTokenCount(value.todayTokens)) {
    return null;
  }
  if ("provider" in value && typeof value.provider !== "string") return null;

  const summaryTokens = optionalTokenFields(value);
  const summaryDate = optionalDateFields(value);
  if (summaryTokens === null || summaryDate === null) return null;
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
      !isUsageState(entry.state) ||
      !isSafeTokenCount(entry.todayTokens)
    ) {
      return null;
    }

    const tokens = optionalTokenFields(entry);
    const date = optionalDateFields(entry);
    const sessions = parseSessionSummaries(entry.sessions);
    if (tokens === null || date === null || sessions === null) return null;

    seenProviders.add(entry.provider);
    providers.push({
      provider: entry.provider,
      state: entry.state,
      ...tokens,
      todayTokens: entry.todayTokens,
      ...date,
      sessions,
    });
  }

  if (seenProviders.size !== providerOrder.length) return null;

  return {
    state: value.state,
    ...(typeof value.provider === "string" ? { provider: value.provider } : {}),
    ...summaryTokens,
    todayTokens: value.todayTokens,
    ...summaryDate,
    sourceHealth,
    providers,
  };
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
