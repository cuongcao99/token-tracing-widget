import { hasExactKeys, isRecord } from "./validation";

export interface UpdateSettingsSnapshot {
  autoUpdate: boolean;
}

const snapshotKeys = ["autoUpdate"] as const;

export function parseUpdateSettings(
  value: unknown,
): UpdateSettingsSnapshot | null {
  if (!isRecord(value) || !hasExactKeys(value, snapshotKeys)) return null;
  if (typeof value.autoUpdate !== "boolean") return null;
  return { autoUpdate: value.autoUpdate };
}
