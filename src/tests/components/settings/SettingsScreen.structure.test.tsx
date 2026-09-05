import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import SettingsScreen from "../../../components/settings/SettingsScreen";
import surfaceStyles from "../../../styles/settings/surface.module.css";

const {
  controller,
  activity,
  startDragging,
  startResizeDragging,
  closeWindow,
  useSettingsController,
} = vi.hoisted(() => {
  const sources = {
    claude: { provider: "claude", enabled: true, windowsRoot: null, wslRoot: null },
    codex: { provider: "codex", enabled: true, windowsRoot: null, wslRoot: null },
  } as const;
  const startDragging = vi.fn().mockResolvedValue(undefined);
  const startResizeDragging = vi.fn().mockResolvedValue(undefined);
  const closeWindow = vi.fn().mockResolvedValue(undefined);
  const controller = {
    closeSettings: vi.fn(() => closeWindow()),
    darkMode: true,
    autoUpdate: false,
    loadingUpdateSettings: false,
    theme: "claude" as const,
    error: null,
    loadingSources: false,
    onDarkModeToggle: vi.fn(),
    onAutoUpdateToggle: vi.fn(),
    onThemeToggle: vi.fn(),
    onProviderVisibilityToggle: vi.fn(),
    onSourceRootChoose: vi.fn(),
    onSourceRootChange: vi.fn(),
    onSourceRootClear: vi.fn(),
    onSourceToggle: vi.fn(),
    sources,
    visible: { claude: true, codex: true },
    widgetError: null,
  };
  const activity = {
    summary: {
      state: "active",
      todayTokens: 0,
      sourceHealth: [
        { provider: "claude", state: "detected" },
        { provider: "codex", state: "detected" },
      ],
      providers: [
        { provider: "claude", state: "idle", todayTokens: 0, sessions: [] },
        { provider: "codex", state: "active", todayTokens: 0, sessions: [] },
      ],
    },
    providerStatuses: [
      { provider: "claude", state: "idle", updated: "No updates yet" },
      { provider: "codex", state: "active", updated: "No updates yet" },
    ],
  };
  return {
    controller,
    activity,
    startDragging,
    startResizeDragging,
    closeWindow,
    useSettingsController: vi.fn(() => controller),
  };
});

vi.mock("../../../hooks/useSettingsController", () => ({
  default: useSettingsController,
}));
vi.mock("../../../hooks/useSettingsActivity", () => ({
  default: vi.fn(() => activity),
  useSettingsActivity: vi.fn(() => activity),
}));
vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: vi.fn(() => ({
    startDragging,
    startResizeDragging,
    close: closeWindow,
  })),
}));

describe("SettingsScreen structure", () => {
  beforeEach(() => {
    controller.darkMode = true;
    controller.theme = "claude";
  });

  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
  });

  it("keeps the approved hierarchy and native window controls", () => {
    render(<SettingsScreen />);
    const main = screen.getByRole("main");

    expect(main).toHaveAttribute("data-theme", "claude");
    expect(main).toHaveAttribute("data-color-mode", "dark");
    expect(main).toHaveClass(surfaceStyles.root);
    expect(main.querySelector(`.${surfaceStyles.body}`)).toBeInTheDocument();
    expect(screen.getByRole("banner")).toBeInTheDocument();
    expect(screen.queryByText("Choose what stays visible.")).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Close settings" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Move settings window" })).toBeInTheDocument();
    expect(screen.getAllByTestId("window-grip-dot")).toHaveLength(6);
    expect(screen.getAllByRole("button", { name: /Resize settings from/ })).toHaveLength(8);
    expect(screen.getByRole("heading", { name: "Visible providers" })).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Sources" })).toBeInTheDocument();
    const changeSource = screen.getByRole("button", { name: "Change source" });
    expect(changeSource).toBeInTheDocument();
    expect(changeSource.closest(`.${surfaceStyles.card}`)).toBeNull();
    expect(screen.queryByRole("button", { name: "Change Claude source" })).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Change Codex source" })).not.toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Appearance" })).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Updates" })).toBeInTheDocument();
    expect(screen.getByRole("switch", { name: "Automatic updates" })).toBeInTheDocument();
    expect(screen.queryByRole("heading", { name: "Agent tracing" })).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Save changes" })).not.toBeInTheDocument();
    expect(screen.getByRole("banner").querySelector(`.${surfaceStyles.heading}`))
      .not.toHaveAttribute("data-tauri-drag-region");
    expect(screen.queryByText("Shape the overlay around your work.")).not.toBeInTheDocument();
    expect(screen.queryByText("Automatic")).not.toBeInTheDocument();
    expect(screen.queryByText("Local only")).not.toBeInTheDocument();
  });

  it("starts native dragging and resizing only from their dedicated controls", async () => {
    render(<SettingsScreen />);
    const header = screen.getByRole("banner");
    const closeButton = screen.getByRole("button", { name: "Close settings" });
    const grip = screen.getByRole("button", { name: "Move settings window" });

    fireEvent.mouseDown(header, { button: 0 });
    expect(startDragging).not.toHaveBeenCalled();
    fireEvent.mouseDown(grip, { button: 0 });
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

  it("tracks the color mode on the settings root", () => {
    const view = render(<SettingsScreen />);
    controller.darkMode = false;
    view.rerender(<SettingsScreen />);

    const main = screen.getByRole("main");
    expect(main).toHaveAttribute("data-theme", "claude");
    expect(main).toHaveAttribute("data-color-mode", "light");
    expect(main).toHaveClass(surfaceStyles.root);
  });
});
