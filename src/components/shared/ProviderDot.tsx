import type { CSSProperties } from "react";
import claudeLogo from "../../assets/providers/claude-spark.svg";
import openAiLogo from "../../assets/providers/openai-monoblossom.svg";
import { providerMeta, type ProviderId } from "../../lib/provider";

interface ProviderDotProps {
  provider: ProviderId;
}

export default function ProviderDot({ provider }: ProviderDotProps) {
  const logo = provider === "claude" ? claudeLogo : openAiLogo;
  const style = {
    "--provider-accent": providerMeta[provider].accent,
  } as CSSProperties;
  return (
    <span
      className={`provider-dot provider-dot--${provider}`}
      style={style}
      aria-hidden="true"
    >
      <img src={logo} alt="" />
    </span>
  );
}
