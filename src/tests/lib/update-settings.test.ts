import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import { parseUpdateSettings } from "../../lib/contracts/update-settings";
import {
  getUpdateSettings,
  saveUpdateSettings,
} from "../../lib/update-settings";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

const validSettings = { autoUpdate: true };

beforeEach(() => vi.mocked(invoke).mockReset());

describe("update settings contract", () => {
  it("accepts the minimal automatic-update preference and rejects extra data", () => {
    expect(parseUpdateSettings({ autoUpdate: false })).toEqual({ autoUpdate: false });
    expect(parseUpdateSettings({ autoUpdate: true })).toEqual({ autoUpdate: true });
    expect(parseUpdateSettings({ autoUpdate: false, endpoint: "private" })).toBeNull();
    expect(parseUpdateSettings({ autoUpdate: "true" })).toBeNull();
  });

  it("gets and validates the persisted preference through the desktop bridge", async () => {
    vi.mocked(invoke).mockResolvedValue(validSettings);

    await expect(getUpdateSettings()).resolves.toEqual(validSettings);
    expect(invoke).toHaveBeenCalledWith("get_update_settings");
  });

  it("sends only the typed preference payload when saving", async () => {
    vi.mocked(invoke).mockResolvedValue(validSettings);

    await saveUpdateSettings(validSettings);

    expect(invoke).toHaveBeenCalledWith("save_update_settings", {
      settings: validSettings,
    });
  });

  it("rejects unsafe values returned by the native bridge", async () => {
    vi.mocked(invoke).mockResolvedValue({ autoUpdate: true, endpoint: "private" });

    await expect(getUpdateSettings()).rejects.toThrow("invalid_update_settings");
  });
});
