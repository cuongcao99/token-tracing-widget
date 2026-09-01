import { isProviderId, providerOrder, type ProviderId } from "../provider";
import { hasExactKeys, isRecord } from "./validation";

export type TraceHookState = "not_installed" | "configured";

export interface TraceHookStatus {
  provider: ProviderId;
  state: TraceHookState;
  requiresTrust: boolean;
}

export interface TraceHooksSnapshot {
  providers: TraceHookStatus[];
}

const snapshotKeys = ["providers"] as const;
const statusKeys = ["provider", "state", "requiresTrust"] as const;

export function parseTraceHooks(value: unknown): TraceHooksSnapshot | null {
  if (!isRecord(value) || !hasExactKeys(value, snapshotKeys)) return null;
  if (!Array.isArray(value.providers) || value.providers.length !== providerOrder.length) {
    return null;
  }

  const seen = new Set<ProviderId>();
  const providers: TraceHookStatus[] = [];
  for (const entry of value.providers) {
    if (
      !isRecord(entry) ||
      !hasExactKeys(entry, statusKeys) ||
      !isProviderId(entry.provider) ||
      seen.has(entry.provider) ||
      (entry.state !== "not_installed" && entry.state !== "configured") ||
      typeof entry.requiresTrust !== "boolean"
    ) {
      return null;
    }
    seen.add(entry.provider);
    providers.push({
      provider: entry.provider,
      state: entry.state,
      requiresTrust: entry.requiresTrust,
    });
  }

  return seen.size === providerOrder.length ? { providers } : null;
}
