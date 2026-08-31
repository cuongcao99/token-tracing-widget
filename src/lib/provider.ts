import claudeLogo from "../assets/providers/claude-spark.svg";
import codexLogo from "../assets/providers/openai-monoblossom.svg";

export type ProviderLogoVariant = "warm-mark" | "monochrome-mark";

export interface ProviderIdentity {
  name: string;
  displayName: string;
  logoSrc: string;
  logoVariant: ProviderLogoVariant;
  fontRole: "display" | "ui";
  accent: string;
}

const providerDefinitions = [
  {
    id: "claude",
    name: "Claude Code",
    displayName: "Claude",
    logoSrc: claudeLogo,
    logoVariant: "warm-mark",
    fontRole: "display",
    accent: "#cc785c",
    automaticRoot: ".claude/projects",
    displayRoot: "~/.claude/projects",
  },
  {
    id: "codex",
    name: "Codex",
    displayName: "Codex",
    logoSrc: codexLogo,
    logoVariant: "monochrome-mark",
    fontRole: "ui",
    accent: "#7e9bff",
    automaticRoot: ".codex/sessions",
    displayRoot: "~/.codex/sessions",
  },
] as const satisfies readonly (ProviderIdentity & {
  id: string;
  automaticRoot: string;
  displayRoot: string;
})[];

export type ProviderId = (typeof providerDefinitions)[number]["id"];

export interface ProviderRegistration extends ProviderIdentity {
  id: ProviderId;
  automaticRoot: string;
  displayRoot: string;
}

export const providerRegistry = providerDefinitions;

export type ProviderMeta = Omit<ProviderRegistration, "id">;

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
