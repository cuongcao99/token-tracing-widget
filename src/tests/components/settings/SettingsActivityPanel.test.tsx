import { act, cleanup, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import SettingsActivityPanel from "../../../components/settings/SettingsActivityPanel";
import type { UsageState, UsageSummary } from "../../../lib/usage-summary";
import type { SourceFormValues } from "../../../components/settings/settings-model";

const state = vi.hoisted(() => ({
  usageListener: undefined as ((summary: UsageSummary) => void) | undefined,
  listenForUsageSummary: vi.fn(),
  getUsageSummary: vi.fn(),
}));

vi.mock("../../../lib/usage-summary", async () => ({
  ...(await vi.importActual<typeof import("../../../lib/usage-summary")>("../../../lib/usage-summary")),
  getUsageSummary: state.getUsageSummary,
  listenForUsageSummary: state.listenForUsageSummary,
}));

const sources: SourceFormValues = {
  claude: { provider: "claude", enabled: true, rootOverride: null },
  codex: { provider: "codex", enabled: true, rootOverride: null },
};

function activitySummary(
  providerState: UsageState,
  lastUpdatedAt?: string,
  sourceState = "detected",
): UsageSummary {
  return {
    state: providerState,
    todayTokens: 10,
    sourceHealth: [
      { provider: "claude", state: sourceState },
      { provider: "codex", state: sourceState },
    ],
    providers: [
      { provider: "claude", state: providerState, todayTokens: 5, lastUpdatedAt, sessions: [] },
      { provider: "codex", state: providerState, todayTokens: 5, lastUpdatedAt, sessions: [] },
    ],
  };
}

beforeEach(() => {
  state.usageListener = undefined;
  state.listenForUsageSummary.mockImplementation(async (listener) => {
    state.usageListener = listener;
    return vi.fn();
  });
  state.getUsageSummary.mockImplementation(() => new Promise(() => undefined));
});

afterEach(() => {
  cleanup();
  vi.clearAllMocks();
});

function renderPanel() {
  return render(
    <SettingsActivityPanel
      visible={{ claude: true, codex: true }}
      onProviderVisibilityToggle={vi.fn()}
      sources={sources}
      onSourceToggle={vi.fn()}
      onSourceRootChoose={vi.fn()}
    />,
  );
}

describe("SettingsActivityPanel", () => {
  it("subscribes internally and preserves every activity status label", async () => {
    renderPanel();
    await waitFor(() => expect(state.usageListener).toBeDefined());

    expect(screen.getAllByText("Loading · No updates yet")).toHaveLength(2);
    const now = new Date().toISOString();
    act(() => state.usageListener!(activitySummary("active", now)));
    expect(await screen.findAllByText("Active · just now")).toHaveLength(2);

    act(() =>
      state.usageListener!(
        activitySummary("idle", new Date(Date.now() - 2 * 60_000).toISOString()),
      ),
    );
    expect(await screen.findAllByText("Idle · 2 min ago")).toHaveLength(2);

    act(() => state.usageListener!(activitySummary("unavailable")));
    expect(await screen.findAllByText("Unavailable · No updates yet")).toHaveLength(2);

    act(() =>
      state.usageListener!(
        activitySummary("stale", new Date(Date.now() - 2 * 60 * 60_000).toISOString()),
      ),
    );
    expect(await screen.findAllByText("Stale · 2 hr ago")).toHaveLength(2);
  });

  it("uses the activity summary for source health in the same panel", async () => {
    renderPanel();
    await waitFor(() => expect(state.usageListener).toBeDefined());

    act(() => state.usageListener!(activitySummary("active", undefined, "permission_denied")));
    expect(await screen.findAllByText("Needs access")).toHaveLength(2);
  });
});
