import { invoke } from "@tauri-apps/api/core";
import { isProviderId, providerOrder, type ProviderId } from "./provider";

export type { ProviderId } from "./provider";

export interface SourceSettings {
  provider: ProviderId;
  enabled: boolean;
  rootOverride: string | null;
}

export interface SourceSettingsSnapshot {
  sources: SourceSettings[];
}

const providerIds = providerOrder;
const snapshotKeys = ["sources"] as const;
const sourceKeys = ["provider", "enabled", "rootOverride"] as const;

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function hasExactKeys(
  value: Record<string, unknown>,
  allowed: readonly string[],
): boolean {
  const keys = Object.keys(value);
  return (
    keys.length === allowed.length &&
    keys.every((key) => allowed.some((name) => name === key))
  );
}

export function parseSourceSettings(
  value: unknown,
): SourceSettingsSnapshot | null {
  if (!isRecord(value) || !hasExactKeys(value, snapshotKeys)) return null;
  if (!Array.isArray(value.sources) || value.sources.length !== providerIds.length) {
    return null;
  }

  const seen = new Set<ProviderId>();
  const sources: SourceSettings[] = [];
  for (const entry of value.sources) {
    if (!isRecord(entry) || !hasExactKeys(entry, sourceKeys)) return null;
    if (!isProviderId(entry.provider) || seen.has(entry.provider)) return null;
    if (typeof entry.enabled !== "boolean") return null;
    if (
      entry.rootOverride !== null &&
      typeof entry.rootOverride !== "string"
    ) {
      return null;
    }

    seen.add(entry.provider);
    sources.push({
      provider: entry.provider,
      enabled: entry.enabled,
      rootOverride: entry.rootOverride,
    });
  }

  if (seen.size !== providerIds.length) return null;
  return { sources };
}

export async function getSourceSettings(): Promise<SourceSettingsSnapshot> {
  const value = await invoke<unknown>("get_source_settings");
  const settings = parseSourceSettings(value);
  if (!settings) {
    throw new Error("invalid_source_settings");
  }
  return settings;
}

export async function updateSourceSettings(
  settings: SourceSettings,
): Promise<SourceSettingsSnapshot> {
  const value = await invoke<unknown>("update_source_settings", { settings });
  const nextSettings = parseSourceSettings(value);
  if (!nextSettings) {
    throw new Error("invalid_source_settings");
  }
  return nextSettings;
}
