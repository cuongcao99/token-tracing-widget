export const providerRegistry = [
  {
    id: "claude",
    name: "Claude Code",
    displayName: "Claude",
    fontRole: "display",
    accent: "#cc785c",
    automaticRoot: ".claude/projects",
    displayRoot: "~/.claude/projects",
  },
  {
    id: "codex",
    name: "Codex",
    displayName: "Codex",
    fontRole: "ui",
    accent: "#7e9bff",
    automaticRoot: ".codex/sessions",
    displayRoot: "~/.codex/sessions",
  },
] as const;

export type ProviderId = (typeof providerRegistry)[number]["id"];
export type ProviderMeta = Omit<(typeof providerRegistry)[number], "id">;

export const providerOrder = providerRegistry.map(
  ({ id }) => id,
) as ProviderId[];

export const providerMeta = Object.fromEntries(
  providerRegistry.map(({ id, ...meta }) => [id, meta]),
) as Record<ProviderId, ProviderMeta>;

export function isProviderId(value: unknown): value is ProviderId {
  return (
    typeof value === "string" &&
    providerRegistry.some((provider) => provider.id === value)
  );
}
