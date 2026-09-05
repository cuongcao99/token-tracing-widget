import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import UpdatesSection from "../../../components/settings/UpdatesSection";

const mocks = vi.hoisted(() => ({
  useAppUpdates: vi.fn(),
}));

vi.mock("../../../hooks/useAppUpdates", () => ({ default: mocks.useAppUpdates }));

const updateHook = {
  status: "idle" as const,
  currentVersion: null,
  availableVersion: null,
  error: null,
  checkForUpdates: vi.fn(),
  installUpdate: vi.fn(),
};

beforeEach(() => {
  mocks.useAppUpdates.mockReturnValue(updateHook);
});

afterEach(() => {
  cleanup();
  vi.clearAllMocks();
});

describe("UpdatesSection", () => {
  it("exposes the automatic preference and manual check action", () => {
    const onAutoUpdateToggle = vi.fn();
    render(
      <UpdatesSection
        autoUpdate={false}
        loadingSettings={false}
        onAutoUpdateToggle={onAutoUpdateToggle}
      />,
    );

    expect(screen.getByRole("heading", { name: "Updates" })).toBeInTheDocument();
    const toggle = screen.getByRole("switch", { name: "Automatic updates" });
    expect(toggle).not.toBeChecked();
    fireEvent.click(toggle);
    expect(onAutoUpdateToggle).toHaveBeenCalledWith(true);

    fireEvent.click(screen.getByRole("button", { name: "Check for updates" }));
    expect(updateHook.checkForUpdates).toHaveBeenCalledTimes(1);
  });

  it("offers installation only after a manual check finds a version", () => {
    mocks.useAppUpdates.mockReturnValue({
      ...updateHook,
      status: "available",
      currentVersion: "0.1.0",
      availableVersion: "0.2.0",
    });
    render(
      <UpdatesSection
        autoUpdate={true}
        loadingSettings={false}
        onAutoUpdateToggle={vi.fn()}
      />,
    );

    expect(screen.getByText("Version 0.2.0 is available.")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Install update" }));
    expect(updateHook.installUpdate).toHaveBeenCalledTimes(1);
  });
});
