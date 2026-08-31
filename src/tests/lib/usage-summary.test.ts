import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import {
  formatRelativeUpdate,
  getUsageSummary,
  listenForUsageSummary,
  parseUsageSummary,
} from "../../lib/usage-summary";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn() }));

const providers = [
  {
    provider: "claude" as const,
    state: "idle" as const,
    currentSessionTokens: 42_184,
    todayTokens: 147_271_872,
    lastUpdatedAt: "2026-01-01T00:07:00Z",
  },
  {
    provider: "codex" as const,
    state: "active" as const,
    currentSessionTokens: 183_256,
    todayTokens: 26_544_812,
    lastUpdatedAt: "2026-01-01T00:09:55Z",
  },
];

const validSummary = {
  state: "active" as const,
  provider: "Codex",
  currentSessionTokens: 183_256,
  todayTokens: 173_816_684,
  lastUpdatedAt: "2026-01-01T00:09:55Z",
  sourceHealth: [
    { provider: "claude", state: "detected" },
    { provider: "codex", state: "detected" },
  ],
  providers,
};

beforeEach(() => {
  vi.mocked(invoke).mockReset();
  vi.mocked(listen).mockReset();
});

describe("usage summary bridge", () => {
  it("requests and validates the per-provider summary command", async () => {
    vi.mocked(invoke).mockResolvedValue(validSummary);

    await expect(getUsageSummary()).resolves.toEqual(validSummary);
    expect(invoke).toHaveBeenCalledWith("get_usage_summary");
  });

  it("rejects a summary without both provider records or with raw data", () => {
    expect(
      parseUsageSummary({
        ...validSummary,
        providers: [providers[0]],
      }),
    ).toBeNull();
    expect(
      parseUsageSummary({
        ...validSummary,
        rawRecord: "private text",
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
    emit!(validSummary);
    emit!({ ...validSummary, prompt: "secret" });

    expect(onSummary).toHaveBeenCalledTimes(1);
    expect(onSummary).toHaveBeenCalledWith(validSummary);
  });

  it("uses compact editorial relative-time labels", () => {
    const now = Date.parse("2026-01-01T00:10:00Z");

    expect(formatRelativeUpdate(undefined, now)).toBe("No updates yet");
    expect(formatRelativeUpdate("2026-01-01T00:09:30Z", now)).toBe("just now");
    expect(formatRelativeUpdate("2026-01-01T00:05:00Z", now)).toBe("5 min ago");
  });
});
