import { hasExactKeys, isRecord } from "./validation";

export interface UpdateCheckResult {
  currentVersion: string;
  availableVersion: string | null;
}

const resultKeys = ["currentVersion", "availableVersion"] as const;

function isVersion(value: unknown): value is string {
  return typeof value === "string" && value.length > 0 && !/[\u0000-\u001f\u007f]/u.test(value);
}

export function parseUpdateCheckResult(
  value: unknown,
): UpdateCheckResult | null {
  if (!isRecord(value) || !hasExactKeys(value, resultKeys)) return null;
  if (!isVersion(value.currentVersion)) return null;
  if (value.availableVersion !== null && !isVersion(value.availableVersion)) return null;
  return {
    currentVersion: value.currentVersion,
    availableVersion: value.availableVersion,
  };
}
