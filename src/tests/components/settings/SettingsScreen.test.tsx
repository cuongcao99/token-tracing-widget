import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import SettingsScreen from "../../../components/settings/SettingsScreen";
import type { UsageSummary } from "../../../lib/usage-summary";
import type { SourceSettingsSnapshot } from "../../../lib/source-settings";
import type { WidgetSettingsSnapshot } from "../../../lib/widget-settings";
import type { WidgetSettingsPreview } from "../../../lib/widget-settings-preview";
import {
  getSourceSettings,
  updateSourceSettings,
} from "../../../lib/source-settings";
import { updateWidgetSettings } from "../../../lib/widget-settings";

const {
  useUsageSummary,
  useWidgetSettings,
  emitWidgetSettingsPreview,
  getCurrentWindow,
  startDragging,
  startResizeDragging,
  closeWindow,
} = vi.hoisted(() => {
  const startDragging = vi.fn().mockResolvedValue(undefined);
  const startResizeDragging = vi.fn().mockResolvedValue(undefined);
  const closeWindow = vi.fn().mockResolvedValue(undefined);
  const getCurrentWindow = vi.fn(() => ({
    startDragging,
    startResizeDragging,
    close: closeWindow,
  }));
  return {
    useUsageSummary: vi.fn(),
    useWidgetSettings: vi.fn(),
    emitWidgetSettingsPreview: vi.fn().mockResolvedValue(undefined),
    getCurrentWindow,
    startDragging,
    startResizeDragging,
    closeWindow,
  };
});

vi.mock("../../../hooks/useUsageSummary", () => ({ useUsageSummary }));
vi.mock("../../../hooks/useWidgetSettings", () => ({ useWidgetSettings }));
vi.mock("../../../lib/widget-settings-preview", () => ({
  emitWidgetSettingsPreview,
}));
vi.mock("@tauri-apps/api/window", () => ({ getCurrentWindow }));
vi.mock("../../../lib/source-settings", async () => {
  const actual = await vi.importActual<typeof import("../../../lib/source-settings")>(
    "../../../lib/source-settings",
  );
  return {
    ...actual,
    getSourceSettings: vi.fn(),
    updateSourceSettings: vi.fn(),
  };
});
vi.mock("../../../lib/widget-settings", async () => {
  const actual = await vi.importActual<typeof import("../../../lib/widget-settings")>(
    "../../../lib/widget-settings",
  );
  return { ...actual, updateWidgetSettings: vi.fn() };
});

const sourceSnapshot: SourceSettingsSnapshot = {
  sources: [
    { provider: "claude", enabled: true, rootOverride: null },
    { provider: "codex", enabled: true, rootOverride: null },
  ],
};

const widgetSettings: WidgetSettingsSnapshot = {
  darkMode: true,
  visibleProviders: [
    { provider: "claude", visible: true },
    { provider: "codex", visible: true },
  ],
};

const summary: UsageSummary = {
  state: "active",
  todayTokens: 173_816_684,
  sourceHealth: [
    { provider: "claude", state: "detected" },
    { provider: "codex", state: "detected" },
  ],
  providers: [
    { provider: "claude", state: "idle", todayTokens: 147_271_872 },
    { provider: "codex", state: "active", todayTokens: 26_544_812 },
  ],
};

beforeEach(() => {
  vi.mocked(getSourceSettings).mockResolvedValue(sourceSnapshot);
  vi.mocked(updateSourceSettings).mockResolvedValue(sourceSnapshot);
  vi.mocked(updateWidgetSettings).mockResolvedValue(widgetSettings);
  useUsageSummary.mockReturnValue({ summary });
  useWidgetSettings.mockReturnValue({
    settings: widgetSettings,
    persistedSettings: widgetSettings,
  });
});

afterEach(() => {
  cleanup();
  vi.clearAllMocks();
});

describe("SettingsScreen", () => {
  it("renders the approved concise settings hierarchy", async () => {
    render(<SettingsScreen />);

    expect(await screen.findByRole("heading", { name: "Settings" })).toBeInTheDocument();
    expect(screen.getByRole("banner").querySelector(".settings-page__heading")).toHaveAttribute(
      "data-tauri-drag-region",
      "",
    );
    expect(screen.getByText("Choose what stays visible.")).toBeInTheDocument();
    expect(screen.getAllByTestId("window-grip-dot")).toHaveLength(6);
    expect(screen.getAllByRole("button", { name: /Resize settings from/ })).toHaveLength(8);
    expect(screen.getByRole("heading", { name: "Visible providers" })).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Sources" })).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Appearance" })).toBeInTheDocument();
    expect(screen.getByRole("main")).toHaveClass("settings-page--dark");
    expect(screen.queryByText("Shape the overlay around your work.")).not.toBeInTheDocument();
    expect(screen.queryByText("Automatic")).not.toBeInTheDocument();
    expect(screen.queryByText("Local only")).not.toBeInTheDocument();
  });

  it("offers a close control and starts native dragging from the header", async () => {
    render(<SettingsScreen />);

    const header = screen.getByRole("banner");
    const closeButton = screen.getByRole("button", { name: "Close settings" });

    fireEvent.mouseDown(header, { button: 0 });
    expect(startDragging).toHaveBeenCalledTimes(1);

    fireEvent.mouseDown(closeButton, { button: 0 });
    expect(startDragging).toHaveBeenCalledTimes(1);

    fireEvent.mouseDown(screen.getByRole("button", { name: "Resize settings from top edge" }), {
      button: 0,
    });
    expect(startResizeDragging).toHaveBeenCalledWith("North");
    expect(startDragging).toHaveBeenCalledTimes(1);

    fireEvent.click(closeButton);
    await waitFor(() => expect(closeWindow).toHaveBeenCalledTimes(1));
  });

  it("previews dark mode immediately and restores the saved value on close", async () => {
    render(<SettingsScreen />);
    await screen.findByRole("heading", { name: "Settings" });

    fireEvent.click(screen.getByRole("switch", { name: "Dark mode" }));

    expect(screen.getByRole("main")).toHaveClass("settings-page--light");
    expect(emitWidgetSettingsPreview).toHaveBeenCalledWith({
      darkMode: false,
      visibleProviders: [
        { provider: "claude", visible: true },
        { provider: "codex", visible: true },
      ],
      sourceEnabled: [
        { provider: "claude", enabled: true },
        { provider: "codex", enabled: true },
      ],
    } satisfies WidgetSettingsPreview);

    fireEvent.click(screen.getByRole("button", { name: "Close settings" }));
    await waitFor(() =>
      expect(emitWidgetSettingsPreview).toHaveBeenLastCalledWith({
        darkMode: true,
        visibleProviders: [
          { provider: "claude", visible: true },
          { provider: "codex", visible: true },
        ],
        sourceEnabled: [
          { provider: "claude", enabled: true },
          { provider: "codex", enabled: true },
        ],
      }),
    );
    expect(closeWindow).toHaveBeenCalledTimes(1);
  });

  it("surfaces a native close failure instead of swallowing it", async () => {
    closeWindow.mockRejectedValueOnce(new Error("permission_denied"));
    render(<SettingsScreen />);

    fireEvent.click(screen.getByRole("button", { name: "Close settings" }));

    expect(await screen.findByRole("alert")).toHaveTextContent(
      "Could not close Settings.",
    );
  });

  it("keeps display, source collection, and appearance controls independent", async () => {
    render(<SettingsScreen />);
    await screen.findByRole("heading", { name: "Settings" });

    fireEvent.click(screen.getByRole("switch", { name: "Show Codex in widget" }));
    fireEvent.click(screen.getByRole("switch", { name: "Collect Codex source" }));
    fireEvent.click(screen.getByRole("switch", { name: "Dark mode" }));

    expect(screen.getByRole("switch", { name: "Show Codex in widget" })).not.toBeChecked();
    expect(screen.getByRole("switch", { name: "Collect Codex source" })).not.toBeChecked();
    expect(screen.getByRole("main")).toHaveClass("settings-page--light");
    expect(updateWidgetSettings).not.toHaveBeenCalled();
    expect(emitWidgetSettingsPreview).toHaveBeenLastCalledWith({
      darkMode: false,
      visibleProviders: [
        { provider: "claude", visible: true },
        { provider: "codex", visible: false },
      ],
      sourceEnabled: [
        { provider: "claude", enabled: true },
        { provider: "codex", enabled: false },
      ],
    });
  });

  it("restores the complete saved snapshot when edited settings are closed", async () => {
    render(<SettingsScreen />);
    await screen.findByRole("heading", { name: "Settings" });

    fireEvent.click(screen.getByRole("switch", { name: "Show Codex in widget" }));
    fireEvent.click(screen.getByRole("switch", { name: "Collect Codex source" }));
    fireEvent.click(screen.getByRole("switch", { name: "Dark mode" }));
    fireEvent.click(screen.getByRole("button", { name: "Close settings" }));

    await waitFor(() =>
      expect(emitWidgetSettingsPreview).toHaveBeenLastCalledWith({
        darkMode: true,
        visibleProviders: [
          { provider: "claude", visible: true },
          { provider: "codex", visible: true },
        ],
        sourceEnabled: [
          { provider: "claude", enabled: true },
          { provider: "codex", enabled: true },
        ],
      }),
    );
    expect(closeWindow).toHaveBeenCalledTimes(1);
  });

  it("keeps source roots behind Change and normalizes a blank override", async () => {
    render(<SettingsScreen />);
    await screen.findByRole("heading", { name: "Settings" });

    expect(screen.queryByRole("textbox", { name: "Claude Code source root" })).not.toBeInTheDocument();
    fireEvent.click(screen.getAllByRole("button", { name: "Change…" })[0]);
    fireEvent.change(screen.getByRole("textbox", { name: "Claude Code source root" }), {
      target: { value: "   " },
    });
    fireEvent.click(screen.getByRole("button", { name: "Save changes" }));

    await waitFor(() => expect(updateSourceSettings).toHaveBeenCalled());
    expect(updateSourceSettings).toHaveBeenNthCalledWith(1, {
      provider: "claude",
      enabled: true,
      rootOverride: null,
    });
  });

  it("saves source settings and widget preferences together", async () => {
    render(<SettingsScreen />);
    await screen.findByRole("heading", { name: "Settings" });
    fireEvent.click(screen.getByRole("switch", { name: "Show Codex in widget" }));
    fireEvent.click(screen.getByRole("button", { name: "Save changes" }));

    await waitFor(() => expect(updateWidgetSettings).toHaveBeenCalled());
    expect(updateWidgetSettings).toHaveBeenCalledWith({
      darkMode: true,
      visibleProviders: [
        { provider: "claude", visible: true },
        { provider: "codex", visible: false },
      ],
    });
    expect(updateSourceSettings).toHaveBeenCalledTimes(2);
    expect(await screen.findByRole("status")).toHaveTextContent("Saved");
  });
});
