import { beforeEach, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import {
  getSourceSettings,
  parseSourceSettings,
  updateSourceSettings,
} from "./source-settings";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

const validSnapshot = {
  sources: [
    { provider: "claude", enabled: true, rootOverride: null },
    { provider: "codex", enabled: false, rootOverride: "C:\\codex" },
  ],
};

beforeEach(() => vi.mocked(invoke).mockReset());

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

it("rejects a settings payload containing raw session data", () => {
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
});

it("rejects duplicate or missing provider records", () => {
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
