import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { createWidgetViewModel } from "../../lib/widget-view-model";
import type { UsageSummary } from "../../lib/usage-summary";
import type { WidgetSettingsSnapshot } from "../../lib/widget-settings";

const settings: WidgetSettingsSnapshot = {
  darkMode: true,
  theme: "claude",
  visibleProviders: [
    { provider: "claude", visible: true },
    { provider: "codex", visible: true },
  ],
};

const summary: UsageSummary = {
  state: "active",
  provider: "Codex",
  todayTokens: 30,
  sourceHealth: [],
  providers: [
    {
      provider: "codex",
      state: "active",
      currentSessionTokens: 20,
      todayTokens: 20,
      lastUpdatedAt: "2026-01-01T00:09:55Z",
      sessions: [
        {
          id: "codex-id",
          name: "Codex run",
          state: "active",
          todayTokens: 20,
        },
      ],
    },
    {
      provider: "claude",
      state: "idle",
      currentSessionTokens: 10,
      todayTokens: 10,
      lastUpdatedAt: "2026-01-01T00:07:00Z",
      sessions: [
        { id: "claude-id", state: "idle", todayTokens: 10 },
      ],
    },
  ],
};

beforeEach(() => {
  vi.useFakeTimers();
  vi.setSystemTime(new Date("2026-01-01T00:10:00Z"));
});

afterEach(() => vi.useRealTimers());

describe("createWidgetViewModel", () => {
  it("maps summaries in canonical registry order and preserves metrics", () => {
    const model = createWidgetViewModel({
      summary,
      settings,
      previewSourceEnabled: null,
    });

    expect(model.providers.map(({ provider }) => provider)).toEqual([
      "claude",
      "codex",
    ]);
    expect(model.providers[0]).toMatchObject({
      provider: "claude",
      identity: { displayName: "Claude" },
      sessionCount: 1,
      sessions: [
        { id: "claude-id", label: "claude-id", state: "idle", todayTokens: 10 },
      ],
      metrics: {
        sessionTokens: 10,
        todayTokens: 10,
        updatedLabel: "3 min ago",
      },
    });
    expect(model.providers[1].sessions).toEqual([
      {
        id: "codex-id",
        label: "Codex run",
        state: "active",
        todayTokens: 20,
      },
    ]);
    expect(model.providers[1].metrics.updatedLabel).toBe("just now");
    expect(model.totalTokens).toBe(30);
    expect(model.visibleProviderCount).toBe(2);
    expect(model).not.toHaveProperty("session");
    expect(model).not.toHaveProperty("sessionId");
  });

  it("omits hidden providers from the combined total", () => {
    const model = createWidgetViewModel({
      summary,
      settings: {
        ...settings,
        visibleProviders: [
          { provider: "claude", visible: false },
          { provider: "codex", visible: true },
        ],
      },
      previewSourceEnabled: null,
    });

    expect(model.providers.map(({ provider }) => provider)).toEqual(["codex"]);
    expect(model.visibleProviderCount).toBe(1);
    expect(model.totalTokens).toBe(20);
  });

  it("updates a session label in place without changing its stable ID", () => {
    const model = createWidgetViewModel({
      summary: {
        ...summary,
        providers: summary.providers.map((usage) =>
          usage.provider === "codex"
            ? {
                ...usage,
                sessions: usage.sessions.map((session) => ({
                  ...session,
                  name: "Renamed run",
                })),
              }
            : usage,
        ),
      },
      settings,
      previewSourceEnabled: null,
    });

    expect(model.providers[1].sessions[0]).toMatchObject({
      id: "codex-id",
      label: "Renamed run",
      state: "active",
      todayTokens: 20,
    });
  });

  it("excludes preview-disabled providers from the combined total", () => {
    const model = createWidgetViewModel({
      summary,
      settings,
      previewSourceEnabled: { claude: false, codex: true },
    });

    expect(model.totalTokens).toBe(20);
  });
});
