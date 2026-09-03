import { renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { useUsageSummary } from "../../hooks/useUsageSummary";
import type { UsageSummary } from "../../lib/usage-summary";

const { getUsageSummary, listenForUsageSummary } = vi.hoisted(() => ({
  getUsageSummary: vi.fn(),
  listenForUsageSummary: vi.fn(),
}));

vi.mock("../../lib/usage-summary", () => ({
  getUsageSummary,
  listenForUsageSummary,
}));

const summary: UsageSummary = {
  state: "active",
  todayTokens: 20,
  sourceHealth: [],
  providers: [
    { provider: "claude", state: "idle", todayTokens: 20, sessions: [] },
    { provider: "codex", state: "unavailable", todayTokens: 0, sessions: [] },
  ],
};

beforeEach(() => {
  getUsageSummary.mockReset();
  listenForUsageSummary.mockReset();
});

describe("useUsageSummary", () => {
  it("subscribes before the initial command and cleans up on unmount", async () => {
    const calls: string[] = [];
    const unlisten = vi.fn();
    listenForUsageSummary.mockImplementation(async () => {
      calls.push("listen");
      return unlisten;
    });
    getUsageSummary.mockImplementation(async () => {
      calls.push("get");
      return summary;
    });

    const rendered = renderHook(() => useUsageSummary());
    await waitFor(() => expect(rendered.result.current.summary).toEqual(summary));

    expect(calls).toEqual(["listen", "get"]);
    rendered.unmount();
    expect(unlisten).toHaveBeenCalledTimes(1);
  });

  it("keeps the initial loading shape and falls back to unavailable on command failure", async () => {
    getUsageSummary.mockRejectedValue(new Error("bridge_down"));
    listenForUsageSummary.mockResolvedValue(vi.fn());

    const { result } = renderHook(() => useUsageSummary());
    expect(result.current.summary.state).toBe("loading");
    await waitFor(() => expect(result.current.summary.state).toBe("unavailable"));
    expect(result.current.summary.providers).toHaveLength(2);
  });
});
