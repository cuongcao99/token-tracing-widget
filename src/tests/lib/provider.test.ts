import { describe, expect, it } from "vitest";
import {
  isProviderId,
  providerMeta,
  providerOrder,
  providerRegistry,
} from "../../lib/provider";

describe("provider registry", () => {
  it("keeps canonical provider order and metadata in one registry", () => {
    expect(providerRegistry.map((provider) => provider.id)).toEqual([
      "claude",
      "codex",
    ]);
    expect(providerOrder).toEqual(["claude", "codex"]);
    expect(providerMeta.claude.name).toBe("Claude Code");
    expect(providerMeta.claude.displayName).toBe("Claude");
    expect(providerMeta.claude.fontRole).toBe("display");
    expect(providerMeta.codex.automaticRoot).toBe(".codex/sessions");
  });

  it("accepts only registered provider ids", () => {
    expect(isProviderId("claude")).toBe(true);
    expect(isProviderId("private-provider")).toBe(false);
    expect(isProviderId(null)).toBe(false);
  });
});
