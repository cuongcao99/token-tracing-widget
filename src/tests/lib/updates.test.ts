import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import { parseUpdateCheckResult } from "../../lib/contracts/updates";
import { checkForUpdate, installUpdate } from "../../lib/updates";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

beforeEach(() => vi.mocked(invoke).mockReset());

describe("application updates contract", () => {
  it("accepts only safe version metadata", () => {
    expect(parseUpdateCheckResult({
      currentVersion: "0.1.0",
      availableVersion: null,
    })).toEqual({ currentVersion: "0.1.0", availableVersion: null });
    expect(parseUpdateCheckResult({
      currentVersion: "0.1.0",
      availableVersion: "0.2.0",
    })).toEqual({ currentVersion: "0.1.0", availableVersion: "0.2.0" });
    expect(parseUpdateCheckResult({
      currentVersion: "0.1.0",
      availableVersion: null,
      url: "https://example.test/update",
    })).toBeNull();
    expect(parseUpdateCheckResult({
      currentVersion: "0.1.0",
      availableVersion: 2,
    })).toBeNull();
  });

  it("checks through the typed desktop command", async () => {
    const result = { currentVersion: "0.1.0", availableVersion: null };
    vi.mocked(invoke).mockResolvedValue(result);

    await expect(checkForUpdate()).resolves.toEqual(result);
    expect(invoke).toHaveBeenCalledWith("check_for_update");
  });

  it("installs through the typed desktop command", async () => {
    vi.mocked(invoke).mockResolvedValue(undefined);

    await expect(installUpdate()).resolves.toBeUndefined();
    expect(invoke).toHaveBeenCalledWith("install_update");
  });
});
