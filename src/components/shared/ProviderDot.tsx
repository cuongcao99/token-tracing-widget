import type { ProviderId } from "../../lib/provider";

interface ProviderDotProps {
  provider: ProviderId;
}

export default function ProviderDot({ provider }: ProviderDotProps) {
  return <span className={`provider-dot provider-dot--${provider}`} aria-hidden="true" />;
}
