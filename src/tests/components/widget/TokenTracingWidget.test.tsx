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
  syncWidgetWindowHeight,
  useUsageSummary,
  useWidgetSettings,
  providerSectionRender,
} = vi.hoisted(() => ({
  startCurrentWindowDrag: vi.fn().mockResolvedValue(undefined),
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
    },
    {
      provider: "codex",
      state: "active",
      currentSessionTokens: 183_256,
      todayTokens: 26_544_812,
      lastUpdatedAt: "2026-01-01T00:09:55Z",
    },
  ],
};

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
    expect(screen.getByText("Total", { selector: "span" })).toBeInTheDocument();
    expect(screen.getByText("173,816,684", { selector: "strong" })).toBeInTheDocument();
    expect(screen.queryByText("Total today")).not.toBeInTheDocument();
    expect(screen.queryByText("T")).not.toBeInTheDocument();
    expect(syncWidgetWindowHeight).toHaveBeenCalledWith(2);
    expect(screen.getByRole("main")).toHaveAttribute("data-theme", "claude");
    expect(screen.getByRole("main")).toHaveAttribute("data-color-mode", "dark");

    fireEvent.mouseDown(screen.getByRole("banner"), { button: 0 });
    expect(startCurrentWindowDrag).not.toHaveBeenCalled();
    fireEvent.mouseDown(screen.getByRole("button", { name: "Move widget window" }), {
      button: 0,
    });
    expect(startCurrentWindowDrag).toHaveBeenCalledTimes(1);
  });

  it("uses persisted visibility and dark mode without changing the summary totals", () => {
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
    expect(screen.getByText("173,816,684", { selector: "strong" })).toBeInTheDocument();
    expect(syncWidgetWindowHeight).toHaveBeenCalledWith(1);
  });

  it("previews disabled sources as unavailable and recomputes the aggregate total", () => {
    useWidgetSettings.mockReturnValue({
      settings,
      persistedSettings: settings,
      previewSourceEnabled: { claude: false, codex: true },
    });

    render(<TokenTracingWidget />);

    expect(screen.getByRole("heading", { name: "Claude" })).toBeInTheDocument();
    expect(screen.getByText("Unavailable")).toBeInTheDocument();
    expect(screen.getAllByText("26,544,812", { selector: "strong" })).toHaveLength(2);
    expect(screen.getByRole("contentinfo")).toHaveTextContent("26,544,812");
    expect(screen.queryByText("173,816,684", { selector: "strong" })).not.toBeInTheDocument();
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
