import { render, screen } from "@testing-library/react";
import { beforeEach, expect, it, vi } from "vitest";
import App from "./App";
import { getUsageSummary } from "./lib/usage-summary";

vi.mock("./lib/usage-summary", () => ({ getUsageSummary: vi.fn() }));

beforeEach(() => {
  vi.mocked(getUsageSummary).mockResolvedValue({
    state: "active",
    provider: "Claude Code",
    currentSessionTokens: 1234,
    todayTokens: 5678,
    lastUpdatedAt: "2026-08-29T16:45:00Z",
    sourceHealth: [{ provider: "Claude Code", state: "active" }],
  });
});

it("renders the bootstrap summary returned by Rust", async () => {
  render(<App />);
  expect(await screen.findByText("Claude Code")).toBeInTheDocument();
  expect(screen.getByText("Active")).toBeInTheDocument();
  expect(screen.getByText("1,234 tokens")).toBeInTheDocument();
  expect(screen.getByText("Today: 5,678 tokens")).toBeInTheDocument();
});
