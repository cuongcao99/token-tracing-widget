import type { CSSProperties } from "react";
import { providerMeta, type ProviderId } from "../../lib/provider";

interface ProviderDotProps {
  provider: ProviderId;
}

export default function ProviderDot({ provider }: ProviderDotProps) {
  const style = {
    "--provider-accent": providerMeta[provider].accent,
  } as CSSProperties;
  return (
    <span
      className={`provider-dot provider-dot--${provider}`}
      style={style}
      aria-hidden="true"
    />
  );
}
