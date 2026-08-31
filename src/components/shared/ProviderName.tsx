import { providerMeta, type ProviderId } from "../../lib/provider";

interface ProviderNameProps {
  provider: ProviderId;
}

export default function ProviderName({ provider }: ProviderNameProps) {
  const meta = providerMeta[provider];

  return (
    <span
      className={`provider-name provider-name--${provider} provider-name--font-${meta.fontRole}`}
    >
      {meta.displayName}
    </span>
  );
}
