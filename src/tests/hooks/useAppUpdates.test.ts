import { act, cleanup, renderHook, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import useAppUpdates from "../../hooks/useAppUpdates";

const mocks = vi.hoisted(() => ({
  checkForUpdate: vi.fn(),
  installUpdate: vi.fn(),
}));

vi.mock("../../lib/updates", () => ({
  checkForUpdate: mocks.checkForUpdate,
  installUpdate: mocks.installUpdate,
}));

function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((resolvePromise) => {
    resolve = resolvePromise;
  });
  return { promise, resolve };
}

beforeEach(() => {
  mocks.checkForUpdate.mockReset();
  mocks.installUpdate.mockReset();
  mocks.installUpdate.mockResolvedValue(undefined);
});

afterEach(() => cleanup());

describe("useAppUpdates", () => {
  it("reports an available update and starts its explicit installation", async () => {
    const check = deferred<{
      currentVersion: string;
      availableVersion: string | null;
    }>();
    mocks.checkForUpdate.mockReturnValueOnce(check.promise);
    const { result } = renderHook(() => useAppUpdates());

    act(() => void result.current.checkForUpdates());
    expect(result.current.status).toBe("checking");

    check.resolve({ currentVersion: "0.1.0", availableVersion: "0.2.0" });
    await waitFor(() => expect(result.current.status).toBe("available"));
    expect(result.current.availableVersion).toBe("0.2.0");

    await act(async () => result.current.installUpdate());
    expect(mocks.installUpdate).toHaveBeenCalledTimes(1);
    expect(result.current.status).toBe("installing");
  });

  it("distinguishes an up-to-date result from a failed check", async () => {
    const { result } = renderHook(() => useAppUpdates());
    mocks.checkForUpdate.mockResolvedValueOnce({
      currentVersion: "0.1.0",
      availableVersion: null,
    });

    await act(async () => result.current.checkForUpdates());
    expect(result.current.status).toBe("up-to-date");
    expect(result.current.currentVersion).toBe("0.1.0");

    mocks.checkForUpdate.mockRejectedValueOnce(new Error("update_check_failed"));
    await act(async () => result.current.checkForUpdates());
    expect(result.current.status).toBe("error");
    expect(result.current.error).toBe("Could not check for updates.");
  });
});
