import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import SettingsScreen from "../../../components/settings/SettingsScreen";
import type { UsageSummary } from "../../../lib/usage-summary";
import type { SourceSettingsSnapshot } from "../../../lib/source-settings";
import type { WidgetSettingsSnapshot } from "../../../lib/widget-settings";
import type { WidgetSettingsPreview } from "../../../lib/widget-settings-preview";

const mocks = vi.hoisted(() => {
  const startDragging = vi.fn().mockResolvedValue(undefined);
  const startResizeDragging = vi.fn().mockResolvedValue(undefined);
  const closeWindow = vi.fn().mockResolvedValue(undefined);
  return {
    useWidgetSettings: vi.fn(),
    emitPreview: vi.fn(),
    getSourceSettings: vi.fn(),
    pickSourceRoot: vi.fn(),
    updateSourceSettings: vi.fn(),
    updateWidgetSettings: vi.fn(),
    getUsageSummary: vi.fn(),
    listenForUsageSummary: vi.fn(),
    getCurrentWindow: vi.fn(() => ({ startDragging, startResizeDragging, close: closeWindow })),
    startDragging,
    startResizeDragging,
    closeWindow,
  };
});

vi.mock("../../../hooks/useWidgetSettings", () => ({ useWidgetSettings: mocks.useWidgetSettings }));
vi.mock("../../../lib/widget-settings-preview", () => ({ emitWidgetSettingsPreview: mocks.emitPreview }));
vi.mock("../../../lib/source-settings", () => ({
  getSourceSettings: mocks.getSourceSettings,
  pickSourceRoot: mocks.pickSourceRoot,
  updateSourceSettings: mocks.updateSourceSettings,
}));
vi.mock("../../../lib/widget-settings", () => ({ updateWidgetSettings: mocks.updateWidgetSettings }));
vi.mock("../../../lib/usage-summary", async () => ({
  ...(await vi.importActual<typeof import("../../../lib/usage-summary")>("../../../lib/usage-summary")),
  getUsageSummary: mocks.getUsageSummary,
  listenForUsageSummary: mocks.listenForUsageSummary,
}));
vi.mock("@tauri-apps/api/window", () => ({ getCurrentWindow: mocks.getCurrentWindow }));

const sourceSnapshot: SourceSettingsSnapshot = {
  sources: [
    { provider: "claude", enabled: true, rootOverride: null },
    { provider: "codex", enabled: true, rootOverride: null },
  ],
};
const widgetSettings: WidgetSettingsSnapshot = {
  darkMode: true,
  theme: "claude",
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
    { provider: "claude", state: "idle", todayTokens: 147_271_872, sessions: [] },
    { provider: "codex", state: "active", todayTokens: 26_544_812, sessions: [] },
  ],
};

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (error: unknown) => void;
  const promise = new Promise<T>((resolvePromise, rejectPromise) => {
    resolve = resolvePromise;
    reject = rejectPromise;
  });
  return { promise, resolve, reject };
}

beforeEach(() => {
  mocks.getSourceSettings.mockResolvedValue(sourceSnapshot);
  mocks.pickSourceRoot.mockResolvedValue(null);
  mocks.updateSourceSettings.mockResolvedValue(sourceSnapshot);
  mocks.updateWidgetSettings.mockResolvedValue(widgetSettings);
  mocks.emitPreview.mockResolvedValue(undefined);
  mocks.getUsageSummary.mockResolvedValue(summary);
  mocks.listenForUsageSummary.mockResolvedValue(vi.fn());
  mocks.useWidgetSettings.mockReturnValue({ settings: widgetSettings, persistedSettings: widgetSettings });
});

afterEach(() => {
  cleanup();
  vi.clearAllMocks();
});

describe("SettingsScreen behavior", () => {
  it("previews and persists dark mode with the complete preview payload", async () => {
    render(<SettingsScreen />);
    await screen.findByRole("heading", { name: "Settings" });
    fireEvent.click(screen.getByRole("switch", { name: "Dark mode" }));

    expect(screen.getByRole("main")).toHaveAttribute("data-color-mode", "light");
    expect(mocks.emitPreview).toHaveBeenCalledWith({
      theme: "claude",
      darkMode: false,
      visibleProviders: widgetSettings.visibleProviders,
      sourceEnabled: [
        { provider: "claude", enabled: true },
        { provider: "codex", enabled: true },
      ],
    } satisfies WidgetSettingsPreview);
    await waitFor(() => expect(mocks.updateWidgetSettings).toHaveBeenCalledWith({
      theme: "claude",
      darkMode: false,
      visibleProviders: widgetSettings.visibleProviders,
    }));
  });

  it("previews and persists the selected theme without a submit action", async () => {
    render(<SettingsScreen />);
    await screen.findByRole("heading", { name: "Settings" });
    fireEvent.click(screen.getByRole("button", { name: "Theme: Claude" }));
    fireEvent.click(screen.getByRole("option", { name: "Claude" }));

    expect(mocks.emitPreview).toHaveBeenLastCalledWith({
      theme: "claude",
      darkMode: true,
      visibleProviders: widgetSettings.visibleProviders,
      sourceEnabled: [
        { provider: "claude", enabled: true },
        { provider: "codex", enabled: true },
      ],
    });
    await waitFor(() => expect(mocks.updateWidgetSettings).toHaveBeenCalledWith({
      theme: "claude",
      darkMode: true,
      visibleProviders: widgetSettings.visibleProviders,
    }));
  });

  it("keeps visibility, source collection, and appearance independent", async () => {
    render(<SettingsScreen />);
    await screen.findByRole("heading", { name: "Settings" });
    fireEvent.click(screen.getByRole("switch", { name: "Show Codex in widget" }));
    fireEvent.click(screen.getByRole("switch", { name: "Collect data from Codex" }));
    fireEvent.click(screen.getByRole("switch", { name: "Dark mode" }));

    expect(screen.getByRole("switch", { name: "Show Codex in widget" })).not.toBeChecked();
    expect(screen.getByRole("switch", { name: "Collect data from Codex" })).not.toBeChecked();
    expect(screen.getByRole("main")).toHaveAttribute("data-color-mode", "light");
    expect(mocks.emitPreview).toHaveBeenLastCalledWith({
      theme: "claude",
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
    await waitFor(() => expect(mocks.updateSourceSettings).toHaveBeenCalledWith({
      provider: "codex",
      enabled: false,
      rootOverride: null,
    }));
    expect(mocks.updateWidgetSettings).toHaveBeenCalledTimes(2);
  });

  it("updates the selected root from the native picker without a text input", async () => {
    mocks.pickSourceRoot.mockResolvedValueOnce({
      sources: [
        { provider: "claude", enabled: true, rootOverride: null },
        { provider: "codex", enabled: true, rootOverride: "C:\\work\\codex" },
      ],
    });
    render(<SettingsScreen />);
    await screen.findByRole("heading", { name: "Settings" });
    expect(screen.getByRole("button", {
      name: "Choose Claude source folder: ~/.claude/projects",
    })).toHaveTextContent("~/.claude/projects");
    fireEvent.click(screen.getByRole("button", { name: "Choose Codex source folder: ~/.codex/sessions" }));

    await waitFor(() => expect(screen.getByRole("button", {
      name: "Choose Codex source folder: C:\\work\\codex",
    })).toBeInTheDocument());
    expect(mocks.pickSourceRoot).toHaveBeenCalledWith("codex");
    expect(screen.queryByRole("textbox")).not.toBeInTheDocument();
  });

  it("does not replace the preview while closing after an auto-saved edit", async () => {
    render(<SettingsScreen />);
    await screen.findByRole("heading", { name: "Settings" });
    fireEvent.click(screen.getByRole("switch", { name: "Dark mode" }));
    await waitFor(() => expect(mocks.updateWidgetSettings).toHaveBeenCalledTimes(1));
    const previewCallCount = mocks.emitPreview.mock.calls.length;

    fireEvent.click(screen.getByRole("button", { name: "Close settings" }));
    await waitFor(() => expect(mocks.closeWindow).toHaveBeenCalledTimes(1));
    expect(mocks.emitPreview).toHaveBeenCalledTimes(previewCallCount);
  });

  it("waits for queued persistence before closing and keeps the preview", async () => {
    const write = deferred<WidgetSettingsSnapshot>();
    mocks.updateWidgetSettings.mockReturnValueOnce(write.promise);
    render(<SettingsScreen />);
    await screen.findByRole("heading", { name: "Settings" });
    fireEvent.click(screen.getByRole("switch", { name: "Dark mode" }));
    fireEvent.click(screen.getByRole("button", { name: "Close settings" }));
    await Promise.resolve();
    expect(mocks.closeWindow).not.toHaveBeenCalled();
    write.resolve(widgetSettings);

    await waitFor(() => expect(mocks.closeWindow).toHaveBeenCalledTimes(1));
    expect(mocks.emitPreview).toHaveBeenCalledTimes(1);
  });

  it("surfaces persistence and native close errors and has no Save action", async () => {
    mocks.updateWidgetSettings.mockRejectedValueOnce(new Error("settings_write"));
    render(<SettingsScreen />);
    await screen.findByRole("heading", { name: "Settings" });
    fireEvent.click(screen.getByRole("switch", { name: "Dark mode" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("Could not save settings.");
    expect(screen.queryByRole("button", { name: "Save changes" })).not.toBeInTheDocument();

    mocks.closeWindow.mockRejectedValueOnce(new Error("permission_denied"));
    fireEvent.click(screen.getByRole("button", { name: "Close settings" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("Could not close Settings.");
  });
});
