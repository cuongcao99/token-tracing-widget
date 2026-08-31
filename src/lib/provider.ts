export type ProviderId = "claude" | "codex";

export const providerOrder = ["claude", "codex"] as const satisfies readonly ProviderId[];

export interface ProviderMeta {
  name: string;
  accent: string;
  automaticRoot: string;
}

export const providerMeta: Record<ProviderId, ProviderMeta> = {
  claude: {
    name: "Claude Code",
    accent: "#d97757",
    automaticRoot: ".claude/projects",
  },
  codex: {
    name: "Codex",
    accent: "#7e9bff",
    automaticRoot: ".codex/sessions",
  },
};

export function isProviderId(value: unknown): value is ProviderId {
  return typeof value === "string" && providerOrder.includes(value as ProviderId);
}
