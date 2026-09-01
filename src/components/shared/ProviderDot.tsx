import { getProviderIdentity, ProviderBrandDot } from "./ProviderBrand";
import type { ProviderId } from "../../lib/provider";

interface ProviderDotProps {
  provider: ProviderId;
}

export default function ProviderDot({ provider }: ProviderDotProps) {
  return (
    <ProviderBrandDot identity={getProviderIdentity(provider)} />
  );
}
