export type UnknownRecord = Record<string, unknown>;

export function isRecord(value: unknown): value is UnknownRecord {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

export function hasOnlyKeys(
  value: UnknownRecord,
  allowed: readonly string[],
): boolean {
  return Object.keys(value).every((key) => allowed.includes(key));
}

export function hasExactKeys(
  value: UnknownRecord,
  allowed: readonly string[],
): boolean {
  const keys = Object.keys(value);
  return keys.length === allowed.length && hasOnlyKeys(value, allowed);
}

export function isSafeTokenCount(value: unknown): value is number {
  return (
    typeof value === "number" &&
    Number.isSafeInteger(value) &&
    value >= 0
  );
}

export function isValidDateString(value: unknown): value is string {
  return typeof value === "string" && !Number.isNaN(Date.parse(value));
}
