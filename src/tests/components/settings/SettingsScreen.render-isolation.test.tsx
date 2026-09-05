import { act, cleanup, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import SettingsScreen from "../../../components/settings/SettingsScreen";
import type { UsageSummary } from "../../../lib/usage-summary";

const state = vi.hoisted(() => ({
  appearanceRenders: 0,
  usageListener: undefined as ((summary: UsageSummary) => void) | undefined,
  listenForUsageSummary: vi.fn(),
  getUsageSummary: vi.fn(),
  useSettingsController: vi.fn(),
}));

vi.mock("../../../components/settings/AppearanceSection", () => ({
  default: (_props: Record<string, unknown>) => {
    state.appearanceRenders += 1;
    return <section data-testid="appearance-stub">Appearance</section>;
  },
}));
vi.mock("../../../hooks/useSettingsController", () => ({
  default: state.useSettingsController,
}));
vi.mock("../../../lib/usage-summary", async () => ({
  ...(await vi.importActual<typeof import("../../../lib/usage-summary")>("../../../lib/usage-summary")),
  getUsageSummary: state.getUsageSummary,
  listenForUsageSummary: state.listenForUsageSummary,
}));
vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: vi.fn(() => ({
    startDragging: vi.fn(),
    startResizeDragging: vi.fn(),
    close: vi.fn(),
  })),
}));

const summary: UsageSummary = {
  state: "active",
  todayTokens: 0,
  sourceHealth: [
    { provider: "claude", state: "detected" },
    { provider: "codex", state: "detected" },
  ],
  providers: [
    { provider: "claude", state: "idle", todayTokens: 0, lastUpdatedAt: new Date().toISOString(), sessions: [] },
    { provider: "codex", state: "active", todayTokens: 0, lastUpdatedAt: new Date().toISOString(), sessions: [] },
  ],
};

const controller = {
  closeSettings: vi.fn(),
  darkMode: true,
  theme: "claude" as const,
  error: null,
  loadingSources: false,
  onDarkModeToggle: vi.fn(),
  onThemeToggle: vi.fn(),
  onProviderVisibilityToggle: vi.fn(),
  onSourceRootChoose: vi.fn(),
  onSourceRootChange: vi.fn(),
  onSourceRootClear: vi.fn(),
  onSourceToggle: vi.fn(),
  sources: {
    claude: { provider: "claude", enabled: true, windowsRoot: null, wslRoot: null },
    codex: { provider: "codex", enabled: true, windowsRoot: null, wslRoot: null },
  },
  visible: { claude: true, codex: true },
  widgetError: null,
};

beforeEach(() => {
  state.appearanceRenders = 0;
  state.usageListener = undefined;
  state.useSettingsController.mockReturnValue(controller);
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

describe("SettingsScreen render isolation", () => {
  it("does not rerender Appearance when the activity subscription receives usage", async () => {
    render(<SettingsScreen />);
    await waitFor(() => expect(state.usageListener).toBeDefined());
    const appearanceRenders = state.appearanceRenders;

    act(() => state.usageListener!(summary));
    expect(await screen.findByText("Active · just now")).toBeInTheDocument();
    expect(state.appearanceRenders).toBe(appearanceRenders);
  });
});
