import { getProviderIdentity, ProviderBrandName } from "./ProviderBrand";
import type { ProviderId } from "../../lib/provider";

interface ProviderNameProps {
  provider: ProviderId;
}

export default function ProviderName({ provider }: ProviderNameProps) {
  const identity = getProviderIdentity(provider);
  return (
    <ProviderBrandName identity={identity} />
  );
}
