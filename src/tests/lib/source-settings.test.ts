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
    {
      provider: "claude" as const,
      enabled: true,
      windowsRoot: null,
      wslRoot: null,
    },
    {
      provider: "codex" as const,
      enabled: false,
      windowsRoot: "C:\\codex",
      wslRoot: null,
    },
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
        { provider: "claude", enabled: false, windowsRoot: null, wslRoot: null },
        { provider: "codex", enabled: true, windowsRoot: null, wslRoot: null },
      ],
    });

    await updateSourceSettings({
      provider: "claude",
      enabled: false,
      windowsRoot: null,
      wslRoot: null,
    });

    expect(invoke).toHaveBeenCalledWith("update_source_settings", {
      settings: {
        provider: "claude",
        enabled: false,
        windowsRoot: null,
        wslRoot: null,
      },
    });
  });

  it("opens a folder picker for the selected platform through the native bridge", async () => {
    vi.mocked(invoke).mockResolvedValue(validSnapshot);

    await expect(pickSourceRoot("codex", "wsl")).resolves.toEqual(validSnapshot);

    expect(invoke).toHaveBeenCalledWith("pick_source_root", {
      provider: "codex",
      platform: "wsl",
    });
  });

  it("returns no update when the native folder picker is cancelled", async () => {
    vi.mocked(invoke).mockResolvedValue(null);

    await expect(pickSourceRoot("claude", "windows")).resolves.toBeNull();
    expect(invoke).toHaveBeenCalledWith("pick_source_root", {
      provider: "claude",
      platform: "windows",
    });
  });

  it("rejects raw data and duplicate or missing provider records", () => {
    expect(
      parseSourceSettings({
        sources: [
          {
            provider: "claude",
            enabled: true,
            windowsRoot: null,
            wslRoot: null,
            prompt: "secret",
          },
          { provider: "codex", enabled: true, windowsRoot: null, wslRoot: null },
        ],
      }),
    ).toBeNull();
    expect(
      parseSourceSettings({
        sources: [
          { provider: "claude", enabled: true, windowsRoot: null, wslRoot: null },
          { provider: "claude", enabled: false, windowsRoot: null, wslRoot: null },
        ],
      }),
    ).toBeNull();
    expect(
      parseSourceSettings({
        sources: [{ provider: "claude", enabled: true, windowsRoot: null, wslRoot: null }],
      }),
    ).toBeNull();
  });
});
