import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { useTraceHooks } from "../../hooks/useTraceHooks";
import type { TraceHooksSnapshot } from "../../lib/trace-hooks";

const mocks = vi.hoisted(() => ({
  getTraceHookStatus: vi.fn(),
  updateTraceHook: vi.fn(),
}));

vi.mock("../../lib/trace-hooks", () => ({
  getTraceHookStatus: mocks.getTraceHookStatus,
  updateTraceHook: mocks.updateTraceHook,
}));

const initialSnapshot: TraceHooksSnapshot = {
  providers: [
    { provider: "claude", state: "not_installed", requiresTrust: false },
    { provider: "codex", state: "configured", requiresTrust: true },
  ],
};

const updatedSnapshot: TraceHooksSnapshot = {
  providers: [
    { provider: "claude", state: "configured", requiresTrust: false },
    { provider: "codex", state: "configured", requiresTrust: true },
  ],
};

beforeEach(() => {
  mocks.getTraceHookStatus.mockReset();
  mocks.updateTraceHook.mockReset();
});

describe("useTraceHooks", () => {
  it("loads provider hook status on mount", async () => {
    mocks.getTraceHookStatus.mockResolvedValue(initialSnapshot);

    const { result } = renderHook(() => useTraceHooks());

    expect(result.current.loading).toBe(true);
    expect(result.current.statuses).toEqual([]);

    await waitFor(() => expect(result.current.loading).toBe(false));

    expect(mocks.getTraceHookStatus).toHaveBeenCalledTimes(1);
    expect(result.current.statuses).toEqual(initialSnapshot.providers);
    expect(result.current.error).toBeNull();
  });

  it("exposes a status error without inventing provider state", async () => {
    const error = new Error("hook_status_unavailable");
    mocks.getTraceHookStatus.mockRejectedValue(error);

    const { result } = renderHook(() => useTraceHooks());

    await waitFor(() => expect(result.current.loading).toBe(false));

    expect(result.current.statuses).toEqual([]);
    expect(result.current.error).toBe(error);
  });

  it("updates the snapshot through the typed toggle operation", async () => {
    mocks.getTraceHookStatus.mockResolvedValue(initialSnapshot);
    mocks.updateTraceHook.mockResolvedValue(updatedSnapshot);

    const { result } = renderHook(() => useTraceHooks());
    await waitFor(() => expect(result.current.loading).toBe(false));

    await act(async () => {
      await result.current.toggle("claude", true);
    });

    expect(mocks.updateTraceHook).toHaveBeenCalledWith({
      provider: "claude",
      enabled: true,
    });
    expect(result.current.statuses).toEqual(updatedSnapshot.providers);
    expect(result.current.error).toBeNull();
    expect(result.current.updatingProvider).toBeNull();
  });

  it("exposes toggle errors and keeps the last good snapshot", async () => {
    const error = new Error("hook_config_write");
    mocks.getTraceHookStatus.mockResolvedValue(initialSnapshot);
    mocks.updateTraceHook.mockRejectedValue(error);

    const { result } = renderHook(() => useTraceHooks());
    await waitFor(() => expect(result.current.loading).toBe(false));

    await act(async () => {
      await result.current.toggle("claude", true);
    });

    expect(result.current.statuses).toEqual(initialSnapshot.providers);
    expect(result.current.error).toBe(error);
    expect(result.current.updatingProvider).toBeNull();
  });
});
