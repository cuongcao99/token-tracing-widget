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
    sessions: [
      {
        id: "claude-id",
        name: "Claude run",
        state: "active" as const,
        todayTokens: 12,
      },
      { id: "claude-idle", state: "idle" as const, todayTokens: 8 },
    ],
  },
  {
    provider: "codex" as const,
    state: "active" as const,
    currentSessionTokens: 183_256,
    todayTokens: 26_544_812,
    lastUpdatedAt: "2026-01-01T00:09:55Z",
    sessions: [],
    rateLimits: [
      { windowMinutes: 300, usedPercent: 12, resetsAt: 1_788_367_052 },
      { windowMinutes: 10080, usedPercent: 38, resetsAt: 1_788_748_134 },
    ],
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

  it("accepts session metadata and rejects unsafe or duplicate session records", () => {
    expect(parseUsageSummary(validSummary)?.providers[0].sessions).toEqual(
      validSummary.providers[0].sessions,
    );

    const sessions = validSummary.providers[0].sessions;
    expect(
      parseUsageSummary({
        ...validSummary,
        providers: [
          { ...providers[0], sessions: [{ ...sessions[0], id: "" }] },
          providers[1],
        ],
      }),
    ).toBeNull();
    expect(
      parseUsageSummary({
        ...validSummary,
        providers: [
          {
            ...providers[0],
            sessions: [{ ...sessions[0], todayTokens: Number.MAX_SAFE_INTEGER + 1 }],
          },
          providers[1],
        ],
      }),
    ).toBeNull();
    expect(
      parseUsageSummary({
        ...validSummary,
        providers: [
          {
            ...providers[0],
            sessions: [{ ...sessions[0], rawRecord: "secret" }],
          },
          providers[1],
        ],
      }),
    ).toBeNull();
    expect(
      parseUsageSummary({
        ...validSummary,
        providers: [
          {
            ...providers[0],
            sessions: [sessions[0], { ...sessions[0], todayTokens: 4 }],
          },
          providers[1],
        ],
      }),
    ).toBeNull();
  });

  it("accepts bounded provider rate limits and rejects unsafe values", () => {
    expect(parseUsageSummary(validSummary)?.providers[1].rateLimits).toEqual(
      validSummary.providers[1].rateLimits,
    );

    expect(
      parseUsageSummary({
        ...validSummary,
        providers: [
          providers[0],
          {
            ...providers[1],
            rateLimits: [
              { ...validSummary.providers[1].rateLimits![0], usedPercent: 101 },
            ],
          },
        ],
      }),
    ).toBeNull();
    expect(
      parseUsageSummary({
        ...validSummary,
        providers: [
          providers[0],
          {
            ...providers[1],
            rateLimits: [
              { ...validSummary.providers[1].rateLimits![0], rawRecord: "secret" },
            ],
          },
        ],
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
