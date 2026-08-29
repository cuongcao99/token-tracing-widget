import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import { getUsageSummary } from "./usage-summary";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

describe("getUsageSummary", () => {
  beforeEach(() => vi.mocked(invoke).mockReset());

  it("requests only the summary command", async () => {
    const summary = { state: "loading", todayTokens: 0, sourceHealth: [] };
    vi.mocked(invoke).mockResolvedValue(summary);

    await expect(getUsageSummary()).resolves.toEqual(summary);
    expect(invoke).toHaveBeenCalledWith("get_usage_summary");
  });
});
