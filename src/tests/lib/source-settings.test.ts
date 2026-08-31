import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import {
  getSourceSettings,
  pickSourceRoot,
  parseSourceSettings,
  updateSourceSettings,
} from "../../lib/source-settings";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

const validSnapshot = {
  sources: [
    { provider: "claude" as const, enabled: true, rootOverride: null },
    { provider: "codex" as const, enabled: false, rootOverride: "C:\\codex" },
  ],
};

beforeEach(() => vi.mocked(invoke).mockReset());

describe("source settings bridge", () => {
  it("requests and validates the typed settings snapshot", async () => {
    vi.mocked(invoke).mockResolvedValue(validSnapshot);

    await expect(getSourceSettings()).resolves.toEqual(validSnapshot);
    expect(invoke).toHaveBeenCalledWith("get_source_settings");
  });

  it("sends only the source settings payload", async () => {
    vi.mocked(invoke).mockResolvedValue({
      sources: [
        { provider: "claude", enabled: false, rootOverride: null },
        { provider: "codex", enabled: true, rootOverride: null },
      ],
    });

    await updateSourceSettings({
      provider: "claude",
      enabled: false,
      rootOverride: null,
    });

    expect(invoke).toHaveBeenCalledWith("update_source_settings", {
      settings: { provider: "claude", enabled: false, rootOverride: null },
    });
  });

  it("opens a folder picker for the selected provider through the native bridge", async () => {
    vi.mocked(invoke).mockResolvedValue(validSnapshot);

    await expect(pickSourceRoot("codex")).resolves.toEqual(validSnapshot);

    expect(invoke).toHaveBeenCalledWith("pick_source_root", {
      provider: "codex",
    });
  });

  it("returns no update when the native folder picker is cancelled", async () => {
    vi.mocked(invoke).mockResolvedValue(null);

    await expect(pickSourceRoot("claude")).resolves.toBeNull();
    expect(invoke).toHaveBeenCalledWith("pick_source_root", {
      provider: "claude",
    });
  });

  it("rejects raw data and duplicate or missing provider records", () => {
    expect(
      parseSourceSettings({
        sources: [
          {
            provider: "claude",
            enabled: true,
            rootOverride: null,
            prompt: "secret",
          },
          { provider: "codex", enabled: true, rootOverride: null },
        ],
      }),
    ).toBeNull();
    expect(
      parseSourceSettings({
        sources: [
          { provider: "claude", enabled: true, rootOverride: null },
          { provider: "claude", enabled: false, rootOverride: null },
        ],
      }),
    ).toBeNull();
    expect(
      parseSourceSettings({
        sources: [{ provider: "claude", enabled: true, rootOverride: null }],
      }),
    ).toBeNull();
  });
});
