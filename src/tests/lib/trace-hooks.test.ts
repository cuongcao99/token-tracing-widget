import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import {
  getTraceHookStatus,
  parseTraceHooks,
  updateTraceHook,
} from "../../lib/trace-hooks";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

const validSnapshot = {
  providers: [
    { provider: "claude" as const, state: "configured" as const, requiresTrust: false },
    { provider: "codex" as const, state: "not_installed" as const, requiresTrust: false },
  ],
};

beforeEach(() => vi.mocked(invoke).mockReset());

describe("trace hooks bridge", () => {
  it("requests and validates hook status without accepting paths", async () => {
    vi.mocked(invoke).mockResolvedValue(validSnapshot);

    await expect(getTraceHookStatus()).resolves.toEqual(validSnapshot);
    expect(invoke).toHaveBeenCalledWith("get_trace_hook_status");
    expect(
      parseTraceHooks({
        ...validSnapshot,
        configPath: "C:\\private\\settings.json",
      }),
    ).toBeNull();
  });

  it("sends only provider and enabled state to the native boundary", async () => {
    vi.mocked(invoke).mockResolvedValue(validSnapshot);

    await updateTraceHook({ provider: "codex", enabled: true });

    expect(invoke).toHaveBeenCalledWith("update_trace_hook", {
      settings: { provider: "codex", enabled: true },
    });
  });

  it("rejects malformed, duplicate, incomplete, and raw hook status", () => {
    expect(
      parseTraceHooks({
        providers: [
          { provider: "claude", state: "configured", requiresTrust: false, prompt: "secret" },
          { provider: "codex", state: "not_installed", requiresTrust: false },
        ],
      }),
    ).toBeNull();
    expect(
      parseTraceHooks({
        providers: [
          { provider: "claude", state: "configured", requiresTrust: false },
          { provider: "claude", state: "not_installed", requiresTrust: false },
        ],
      }),
    ).toBeNull();
    expect(
      parseTraceHooks({
        providers: [{ provider: "claude", state: "configured", requiresTrust: false }],
      }),
    ).toBeNull();
  });
});
