import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import TokenTracingWidget from "../../../components/widget/TokenTracingWidget";
import type { UsageSummary } from "../../../lib/usage-summary";
import type { WidgetSettingsSnapshot } from "../../../lib/widget-settings";

const {
  startCurrentWindowDrag,
  syncWidgetWindowHeight,
  useUsageSummary,
  useWidgetSettings,
} = vi.hoisted(() => ({
  startCurrentWindowDrag: vi.fn().mockResolvedValue(undefined),
  syncWidgetWindowHeight: vi.fn().mockResolvedValue(undefined),
  useUsageSummary: vi.fn(),
  useWidgetSettings: vi.fn(),
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
  visibleProviders: [
    { provider: "claude", visible: true },
    { provider: "codex", visible: true },
  ],
};

beforeEach(() => {
  startCurrentWindowDrag.mockClear();
  syncWidgetWindowHeight.mockClear();
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
    expect(screen.getByRole("status")).toHaveTextContent("Live");
    expect(screen.getByRole("banner")).toHaveClass("widget-header");
    expect(screen.getByRole("banner")).not.toHaveAttribute("data-tauri-drag-region");
    expect(screen.getByRole("button", { name: "Move widget window" })).toBeInTheDocument();
    expect(screen.getAllByTestId("window-grip-dot")).toHaveLength(6);
    expect(screen.getAllByRole("button", { name: /Resize widget from/ })).toHaveLength(8);
    expect(screen.getByText("Claude Code")).toBeInTheDocument();
    expect(screen.getByText("Codex")).toBeInTheDocument();
    expect(screen.getAllByText("Session", { selector: "span" })).toHaveLength(2);
    expect(screen.getAllByText("Today", { selector: "span" })).toHaveLength(2);
    expect(screen.getByText("Total", { selector: "span" })).toBeInTheDocument();
    expect(screen.getByText("173,816,684", { selector: "strong" })).toBeInTheDocument();
    expect(screen.queryByText("Total today")).not.toBeInTheDocument();
    expect(screen.queryByText("T")).not.toBeInTheDocument();
    expect(syncWidgetWindowHeight).toHaveBeenCalledWith(2);

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

    expect(screen.getAllByText("Claude Code")).toHaveLength(1);
    expect(screen.queryByText("Codex")).not.toBeInTheDocument();
    expect(screen.getByRole("main")).toHaveClass("widget--light");
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

    expect(screen.getByText("Claude Code")).toBeInTheDocument();
    expect(screen.getByText("Unavailable")).toBeInTheDocument();
    expect(screen.getAllByText("26,544,812", { selector: "strong" })).toHaveLength(2);
    expect(screen.getByRole("contentinfo")).toHaveTextContent("26,544,812");
    expect(screen.queryByText("173,816,684", { selector: "strong" })).not.toBeInTheDocument();
  });
});
