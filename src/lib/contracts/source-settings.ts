import { isProviderId, providerOrder, type ProviderId } from "../provider";
import { hasExactKeys, isRecord } from "./validation";

export interface SourceSettings {
  provider: ProviderId;
  enabled: boolean;
  windowsRoot: string | null;
  wslRoot: string | null;
}

export type SourcePlatform = "windows" | "wsl";

export interface SourceSettingsSnapshot {
  sources: SourceSettings[];
}

const snapshotKeys = ["sources"] as const;
const sourceKeys = ["provider", "enabled", "windowsRoot", "wslRoot"] as const;

export function parseSourceSettings(
  value: unknown,
): SourceSettingsSnapshot | null {
  if (!isRecord(value) || !hasExactKeys(value, snapshotKeys)) return null;
  if (!Array.isArray(value.sources) || value.sources.length !== providerOrder.length) {
    return null;
  }

  const seen = new Set<ProviderId>();
  const sources: SourceSettings[] = [];
  for (const entry of value.sources) {
    if (!isRecord(entry) || !hasExactKeys(entry, sourceKeys)) return null;
    if (!isProviderId(entry.provider) || seen.has(entry.provider)) return null;
    if (typeof entry.enabled !== "boolean") return null;
    if (entry.windowsRoot !== null && typeof entry.windowsRoot !== "string") {
      return null;
    }
    if (entry.wslRoot !== null && typeof entry.wslRoot !== "string") {
      return null;
    }

    seen.add(entry.provider);
    sources.push({
      provider: entry.provider,
      enabled: entry.enabled,
      windowsRoot: entry.windowsRoot,
      wslRoot: entry.wslRoot,
    });
  }

  if (seen.size !== providerOrder.length) return null;
  return { sources };
}
