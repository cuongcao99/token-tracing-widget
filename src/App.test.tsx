import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import App from "./App";
import {
  formatRelativeUpdate,
  getUsageSummary,
  listenForUsageSummary,
  type UsageSummary,
} from "./lib/usage-summary";

vi.mock("./lib/usage-summary", () => ({
  formatRelativeUpdate: vi.fn(),
  getUsageSummary: vi.fn(),
  listenForUsageSummary: vi.fn(),
}));

beforeEach(() => {
  vi.mocked(formatRelativeUpdate).mockImplementation((lastUpdatedAt) =>
    lastUpdatedAt ? "Updated just now" : "No updates yet",
  );
  vi.mocked(listenForUsageSummary).mockResolvedValue(vi.fn());
  vi.mocked(getUsageSummary).mockResolvedValue({
    state: "active",
    provider: "Claude Code",
    currentSessionTokens: 1234,
    todayTokens: 5678,
    lastUpdatedAt: "2026-08-29T16:45:00Z",
    sourceHealth: [{ provider: "Claude Code", state: "active" }],
  });
});

afterEach(() => {
  cleanup();
});

it("renders the summary returned by Rust", async () => {
  render(<App />);
  expect(await screen.findByText("Claude Code")).toBeInTheDocument();
  expect(screen.getByText("Active")).toBeInTheDocument();
  expect(screen.getByText("1,234 tokens")).toBeInTheDocument();
  expect(screen.getByText("Today: 5,678 tokens")).toBeInTheDocument();
});

it("marks the non-interactive header as a Tauri drag region", () => {
  render(<App />);

  expect(screen.getByRole("banner")).toHaveAttribute(
    "data-tauri-drag-region",
    "",
  );
});

it("updates the overlay from a valid summary event and cleans up the listener", async () => {
  let onSummary: ((summary: UsageSummary) => void) | undefined;
  const unlisten = vi.fn();
  vi.mocked(listenForUsageSummary).mockImplementation(async (callback) => {
    onSummary = callback;
    return unlisten;
  });

  const { unmount } = render(<App />);
  expect(await screen.findByText("Claude Code")).toBeInTheDocument();

  onSummary?.({
    state: "active",
    provider: "Codex",
    currentSessionTokens: 20,
    todayTokens: 40,
    lastUpdatedAt: "2026-08-30T00:00:01Z",
    sourceHealth: [{ provider: "codex", state: "detected" }],
  });

  expect(await screen.findByText("Codex")).toBeInTheDocument();
  expect(screen.getByText("20 tokens")).toBeInTheDocument();
  expect(screen.getByText("Today: 40 tokens")).toBeInTheDocument();

  unmount();
  expect(unlisten).toHaveBeenCalledTimes(1);
});

it("shows unavailable when the initial summary is invalid", async () => {
  vi.mocked(getUsageSummary).mockRejectedValueOnce(
    new Error("invalid_usage_summary"),
  );

  render(<App />);

  expect(
    await screen.findByRole("heading", { name: "Token Tracing" }),
  ).toBeInTheDocument();
  expect(screen.getAllByText("Unavailable")).toHaveLength(2);
});
