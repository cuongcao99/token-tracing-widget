import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import {
  formatRelativeUpdate,
  getUsageSummary,
  listenForUsageSummary,
  parseUsageSummary,
} from "./usage-summary";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn() }));

describe("getUsageSummary", () => {
  beforeEach(() => vi.mocked(invoke).mockReset());

  it("requests only the summary command", async () => {
    const summary = { state: "loading", todayTokens: 0, sourceHealth: [] };
    vi.mocked(invoke).mockResolvedValue(summary);

    await expect(getUsageSummary()).resolves.toEqual(summary);
    expect(invoke).toHaveBeenCalledWith("get_usage_summary");
  });

  it("rejects an invalid command payload", async () => {
    vi.mocked(invoke).mockResolvedValue({
      state: "active",
      todayTokens: 20,
      sourceHealth: [],
      rawRecord: "private text",
    });

    await expect(getUsageSummary()).rejects.toThrow("invalid_usage_summary");
  });
});

it("rejects a summary carrying a forbidden raw field", () => {
  expect(
    parseUsageSummary({
      state: "active",
      todayTokens: 20,
      sourceHealth: [],
      prompt: "private text",
    }),
  ).toBeNull();
});

it("forwards only valid summary-changed payloads", async () => {
  const onSummary = vi.fn();
  const stop = vi.fn();
  let emit: ((payload: unknown) => void) | undefined;
  vi.mocked(listen).mockImplementation(async (_event, handler) => {
    emit = (payload) => handler({ payload } as Parameters<typeof handler>[0]);
    return stop;
  });

  await listenForUsageSummary(onSummary);
  emit!({ state: "active", todayTokens: 20, sourceHealth: [] });
  emit!({ state: "active", todayTokens: 20, sourceHealth: [], rawRecord: "secret" });

  expect(onSummary).toHaveBeenCalledTimes(1);
  expect(onSummary).toHaveBeenCalledWith({
    state: "active",
    todayTokens: 20,
    sourceHealth: [],
  });
});

it("formats relative update time without a polling timer", () => {
  const now = Date.parse("2026-01-01T00:10:00Z");

  expect(formatRelativeUpdate(undefined, now)).toBe("No updates yet");
  expect(formatRelativeUpdate("2026-01-01T00:09:30Z", now)).toBe("Updated just now");
  expect(formatRelativeUpdate("2026-01-01T00:05:00Z", now)).toBe("Updated 5 min ago");
});
