import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import TokenTracingWidget from "../../../components/widget/TokenTracingWidget";
import {
  areProviderUsageRowsEqual,
} from "../../../components/widget/ProviderUsageRow";
import type { WidgetProviderRowProps } from "../../../components/widget/widget-types";
import { createWidgetViewModel } from "../../../lib/widget-view-model";
import type { UsageSummary } from "../../../lib/usage-summary";
import type { WidgetSettingsSnapshot } from "../../../lib/widget-settings";

const {
  startCurrentWindowDrag,
  startCurrentWindowResize,
  syncWidgetWindowHeight,
  useUsageSummary,
  useWidgetSettings,
  providerSectionRender,
} = vi.hoisted(() => ({
  startCurrentWindowDrag: vi.fn().mockResolvedValue(undefined),
  startCurrentWindowResize: vi.fn().mockResolvedValue(undefined),
  syncWidgetWindowHeight: vi.fn().mockResolvedValue(undefined),
  useUsageSummary: vi.fn(),
  useWidgetSettings: vi.fn(),
  providerSectionRender: vi.fn(),
}));

vi.mock("../../../hooks/useUsageSummary", () => ({
  useUsageSummary,
}));
vi.mock("../../../hooks/useWidgetSettings", () => ({
  useWidgetSettings,
}));
vi.mock("../../../lib/window-actions", () => ({
  startCurrentWindowDrag,
  startCurrentWindowResize,
}));
vi.mock("../../../lib/window-sizing", () => ({
  syncWidgetWindowHeight,
}));
vi.mock("../../../components/widget/ProviderSection", async () => {
  const actual = await vi.importActual<
    typeof import("../../../components/widget/ProviderSection")
  >("../../../components/widget/ProviderSection");
  providerSectionRender.mockImplementation(actual.default);
  return { ...actual, default: providerSectionRender };
});

const summary: UsageSummary = {
  state: "active",
  provider: "Codex",
  todayTokens: 173_816_684,
  sourceHealth: [],
  providers: [
    {
      provider: "claude",
      state: "idle",
      currentSessionTokens: 42_184,
      todayTokens: 147_271_872,
      lastUpdatedAt: "2026-01-01T00:07:00Z",
      sessions: [
        { id: "claude-run", state: "idle", todayTokens: 147_271_872 },
      ],
    },
    {
      provider: "codex",
      state: "active",
      currentSessionTokens: 183_256,
      todayTokens: 26_544_812,
      lastUpdatedAt: "2026-01-01T00:09:55Z",
      sessions: [
        {
          id: "codex-run",
          name: "Codex run",
          state: "active",
          todayTokens: 26_544_812,
        },
      ],
      rateLimits: [
        { windowMinutes: 300, usedPercent: 12, resetsAt: 1_788_367_052 },
        { windowMinutes: 10080, usedPercent: 38, resetsAt: 1_788_748_134 },
      ],
    },
  ],
} as UsageSummary;

const settings: WidgetSettingsSnapshot = {
  darkMode: true,
  theme: "claude",
  visibleProviders: [
    { provider: "claude", visible: true },
    { provider: "codex", visible: true },
  ],
};

beforeEach(() => {
  startCurrentWindowDrag.mockClear();
  startCurrentWindowResize.mockClear();
  syncWidgetWindowHeight.mockClear();
  providerSectionRender.mockClear();
  useUsageSummary.mockReturnValue({ summary });
  useWidgetSettings.mockReturnValue({
    settings,
    persistedSettings: settings,
    previewSourceEnabled: null,
  });
});

afterEach(() => cleanup());

describe("TokenTracingWidget", () => {
  it("renders both provider rows and the combined total in the approved hierarchy", () => {
    render(<TokenTracingWidget />);

    expect(screen.getByRole("heading", { name: "Token Tracing" })).toBeInTheDocument();
    expect(screen.queryByRole("status")).not.toBeInTheDocument();
    expect(screen.getByRole("banner").className).toMatch(/header/);
    expect(screen.getByRole("banner")).not.toHaveAttribute("data-tauri-drag-region");
    expect(screen.getByRole("button", { name: "Move widget window" })).toBeInTheDocument();
    expect(screen.getAllByTestId("window-grip-dot")).toHaveLength(6);
    expect(screen.getAllByRole("button", { name: /Resize widget from/ })).toHaveLength(8);
    expect(screen.getByRole("heading", { name: "Claude" })).toBeInTheDocument();
    expect(screen.getByText("Codex")).toBeInTheDocument();
    expect(
      screen
        .getByRole("main")
        .querySelector('[data-logo-variant="monochrome-mark"]'),
    ).not.toBeNull();
    expect(
      screen.getByRole("main").querySelector('[data-logo-variant="warm-mark"]'),
    ).not.toBeNull();
    expect(screen.getAllByText("Session", { selector: "span" })).toHaveLength(2);
    expect(screen.getAllByText("Today", { selector: "span" })).toHaveLength(2);
    expect(screen.getAllByLabelText("Session count: 1 session today")).toHaveLength(2);
    expect(screen.queryByText("1 sessions today")).not.toBeInTheDocument();
    expect(screen.getByText("Codex run")).toBeInTheDocument();
    expect(screen.getByText("More sessions")).toBeInTheDocument();
    expect(screen.getByText("5h")).toBeInTheDocument();
    expect(screen.getByText("88%")).toBeInTheDocument();
    expect(
      screen.getByRole("progressbar", { name: "5h: 88% remaining" }),
    ).toHaveAttribute("aria-valuenow", "88");
    expect(screen.getByText("Total", { selector: "span" })).toBeInTheDocument();
    expect(screen.getByLabelText("Total: 173,816,684 tokens")).toBeInTheDocument();
    expect(screen.queryByText("Total today")).not.toBeInTheDocument();
    expect(screen.getByRole("banner").querySelector('[data-state="active"]'))
      .toHaveAttribute("data-phrase");
    expect(syncWidgetWindowHeight).toHaveBeenCalledWith(2, undefined, true);
    expect(screen.getByRole("main")).toHaveAttribute("data-theme", "claude");
    expect(screen.getByRole("main")).toHaveAttribute("data-color-mode", "dark");

    fireEvent.mouseDown(screen.getByRole("banner"), { button: 0 });
    expect(startCurrentWindowDrag).not.toHaveBeenCalled();
    fireEvent.mouseDown(screen.getByRole("button", { name: "Move widget window" }), {
      button: 0,
    });
    expect(startCurrentWindowDrag).toHaveBeenCalledTimes(1);
  });

  it("renders a loading skeleton instead of zero-value content while usage loads", () => {
    const loadingSummary: UsageSummary = {
      ...summary,
      state: "loading",
      provider: undefined,
      currentSessionTokens: undefined,
      todayTokens: 0,
      lastUpdatedAt: undefined,
      providers: summary.providers.map((usage) => ({
        ...usage,
        state: "loading",
        currentSessionTokens: undefined,
        todayTokens: 0,
        lastUpdatedAt: undefined,
        sessions: [],
        rateLimits: [],
      })),
    };
    useUsageSummary.mockReturnValue({ summary: loadingSummary });

    render(<TokenTracingWidget />);

    expect(screen.getByRole("main")).toHaveAttribute("aria-busy", "true");
    expect(
      screen.getByRole("status", { name: "Loading token usage" }),
    ).toBeInTheDocument();
    expect(screen.getAllByTestId("widget-skeleton-provider")).toHaveLength(2);
    expect(screen.getByRole("heading", { name: "Claude" })).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Codex" })).toBeInTheDocument();
    expect(screen.getAllByTestId("widget-skeleton-limit")).toHaveLength(4);
    expect(screen.getAllByTestId("widget-skeleton-metric")).toHaveLength(4);
    expect(screen.getByTestId("widget-skeleton-total")).toBeInTheDocument();
    expect(screen.queryByLabelText("Today's sessions")).not.toBeInTheDocument();
    expect(screen.queryByText("More sessions")).not.toBeInTheDocument();
    expect(screen.queryByText("No activity yet today")).not.toBeInTheDocument();
    expect(screen.queryByText("0", { selector: "strong" })).not.toBeInTheDocument();
  });

  it("uses persisted visibility and shows the visible provider total", () => {
    useWidgetSettings.mockReturnValue({
      settings: {
        ...settings,
        darkMode: false,
        visibleProviders: [
          { provider: "claude", visible: true },
          { provider: "codex", visible: false },
        ],
      },
      persistedSettings: settings,
      previewSourceEnabled: null,
    });

    render(<TokenTracingWidget />);

    expect(screen.getAllByRole("heading", { name: "Claude" })).toHaveLength(1);
    expect(screen.queryByText("Codex")).not.toBeInTheDocument();
    expect(screen.getByRole("main")).toHaveAttribute("data-color-mode", "light");
    expect(screen.getByText("Total", { selector: "span" })).toBeInTheDocument();
    expect(screen.getByRole("contentinfo")).toHaveTextContent("Total");
    expect(screen.getByRole("contentinfo")).toHaveTextContent("147,271,872");
    expect(syncWidgetWindowHeight).toHaveBeenCalledWith(1, undefined, true);
  });

  it("previews disabled sources as unavailable and recomputes the aggregate total", () => {
    useWidgetSettings.mockReturnValue({
      settings,
      persistedSettings: settings,
      previewSourceEnabled: { claude: false, codex: true },
    });

    render(<TokenTracingWidget />);

    expect(screen.getByRole("heading", { name: "Claude" })).toBeInTheDocument();
    expect(screen.queryByText("Unavailable")).not.toBeInTheDocument();
    expect(
      screen
        .getAllByTestId("rolling-number")
        .filter((number) => number.getAttribute("data-value") === "26,544,812"),
    ).toHaveLength(3);
    expect(screen.getByRole("contentinfo")).toHaveTextContent("26,544,812");
    expect(screen.queryByLabelText("Total: 173,816,684 tokens")).not.toBeInTheDocument();
  });

  it("does not resize when a source toggle changes without changing widget content", () => {
    const { rerender } = render(<TokenTracingWidget />);

    expect(syncWidgetWindowHeight).toHaveBeenCalledTimes(1);

    useWidgetSettings.mockReturnValue({
      settings,
      persistedSettings: settings,
      previewSourceEnabled: { claude: false, codex: true },
    });
    rerender(<TokenTracingWidget />);

    const sourceRefreshedSummary: UsageSummary = {
      ...summary,
      providers: summary.providers.map((usage) =>
        usage.provider === "codex"
          ? {
              ...usage,
              state: "unavailable",
              currentSessionTokens: undefined,
              todayTokens: 0,
              sessions: [],
              rateLimits: [],
            }
          : usage,
      ),
    };
    useUsageSummary.mockReturnValue({ summary: sourceRefreshedSummary });
    rerender(<TokenTracingWidget />);

    expect(syncWidgetWindowHeight).toHaveBeenCalledTimes(1);
  });

  it("auto-resizes when visible providers change in Settings", () => {
    const { rerender } = render(<TokenTracingWidget />);

    expect(syncWidgetWindowHeight).toHaveBeenCalledTimes(1);

    useWidgetSettings.mockReturnValue({
      settings: {
        ...settings,
        visibleProviders: [
          { provider: "claude", visible: false },
          { provider: "codex", visible: true },
        ],
      },
      persistedSettings: settings,
      previewSourceEnabled: null,
    });
    rerender(<TokenTracingWidget />);

    expect(syncWidgetWindowHeight).toHaveBeenCalledTimes(2);
  });

  it("measures intrinsic content instead of adding space to the current height", () => {
    const { rerender } = render(<TokenTracingWidget />);
    const root = screen.getByRole("main");
    const providerList = screen.getByRole("region", { name: "Provider usage" });
    const codexSection = screen.getByRole("heading", { name: "Codex" }).closest("article");

    expect(codexSection).not.toBeNull();
    if (!codexSection) return;

    Object.defineProperties(root, {
      clientHeight: { configurable: true, value: 400 },
    });
    Object.defineProperties(providerList, {
      clientHeight: { configurable: true, value: 300 },
      scrollHeight: { configurable: true, value: 300 },
    });
    vi.spyOn(providerList, "getBoundingClientRect").mockReturnValue({
      top: 100,
    } as DOMRect);
    vi.spyOn(codexSection, "getBoundingClientRect").mockReturnValue({
      bottom: 280,
    } as DOMRect);

    useWidgetSettings.mockReturnValue({
      settings: {
        ...settings,
        visibleProviders: [
          { provider: "claude", visible: false },
          { provider: "codex", visible: true },
        ],
      },
      persistedSettings: settings,
      previewSourceEnabled: null,
    });
    rerender(<TokenTracingWidget />);

    expect(syncWidgetWindowHeight).toHaveBeenLastCalledWith(1, 280, true);
  });

  it("auto-resizes when the session content structure changes", () => {
    const { rerender } = render(<TokenTracingWidget />);

    expect(syncWidgetWindowHeight).toHaveBeenCalledTimes(1);

    const nextSummary: UsageSummary = {
      ...summary,
      providers: summary.providers.map((usage) =>
        usage.provider === "codex"
          ? {
              ...usage,
              sessions: [
                ...usage.sessions,
                { id: "codex-idle", state: "idle", todayTokens: 1 },
              ],
            }
          : usage,
      ),
    };
    useUsageSummary.mockReturnValue({ summary: nextSummary });
    rerender(<TokenTracingWidget />);

    expect(syncWidgetWindowHeight).toHaveBeenCalledTimes(2);
  });

  it("stops auto-resizing after a user starts a vertical resize", () => {
    const { rerender } = render(<TokenTracingWidget />);

    expect(syncWidgetWindowHeight).toHaveBeenCalledTimes(1);
    fireEvent.mouseDown(
      screen.getByRole("button", { name: "Resize widget from bottom edge" }),
      { button: 0 },
    );

    const nextSummary: UsageSummary = {
      ...summary,
      providers: summary.providers.map((usage) =>
        usage.provider === "codex"
          ? {
              ...usage,
              sessions: [
                ...usage.sessions,
                { id: "codex-idle", state: "idle", todayTokens: 1 },
              ],
            }
          : usage,
      ),
    };
    useUsageSummary.mockReturnValue({ summary: nextSummary });
    rerender(<TokenTracingWidget />);

    expect(startCurrentWindowResize).toHaveBeenCalledWith("South");
    expect(syncWidgetWindowHeight).toHaveBeenCalledTimes(1);
  });

  it("explains when a provider has no activity today", () => {
    useUsageSummary.mockReturnValue({
      summary: {
        ...summary,
        providers: summary.providers.map((usage) =>
          usage.provider === "claude"
            ? { ...usage, currentSessionTokens: 0, todayTokens: 0, sessions: [] }
            : usage,
        ),
      },
    });

    render(<TokenTracingWidget />);

    expect(screen.getByText("No activity yet today")).toBeInTheDocument();
    expect(screen.getByLabelText("Session count: 0 sessions today")).toBeInTheDocument();
    expect(screen.queryByText("0 sessions today")).not.toBeInTheDocument();
  });

  it("keeps an unchanged provider row memo-stable when only Codex changes", () => {
    const first = createWidgetViewModel({
      summary,
      settings,
      previewSourceEnabled: null,
    });
    const next = createWidgetViewModel({
      summary: {
        ...summary,
        providers: summary.providers.map((usage) =>
          usage.provider === "codex"
            ? { ...usage, todayTokens: usage.todayTokens + 1 }
            : usage,
        ),
      },
      settings,
      previewSourceEnabled: null,
    });

    const claudeProps: WidgetProviderRowProps = { usage: first.providers[0] };
    const nextClaudeProps: WidgetProviderRowProps = { usage: next.providers[0] };
    const nextCodexProps: WidgetProviderRowProps = { usage: next.providers[1] };
    expect(areProviderUsageRowsEqual(claudeProps, nextClaudeProps)).toBe(true);
    expect(areProviderUsageRowsEqual({ usage: first.providers[1] }, nextCodexProps)).toBe(false);
  });

  it("avoids rerendering Claude when a Codex-only summary update arrives", () => {
    const { rerender } = render(<TokenTracingWidget />);
    const nextSummary: UsageSummary = {
      ...summary,
      providers: summary.providers.map((usage) =>
        usage.provider === "codex"
          ? { ...usage, todayTokens: usage.todayTokens + 1 }
          : usage,
      ),
    };

    useUsageSummary.mockReturnValue({ summary: nextSummary });
    rerender(<TokenTracingWidget />);

    const renderedIdentities = providerSectionRender.mock.calls.map(
      (call) =>
        (call[0] as { identity: { displayName: string } }).identity.displayName,
    );
    expect(renderedIdentities).toEqual(["Claude", "Codex", "Codex"]);
  });
});
