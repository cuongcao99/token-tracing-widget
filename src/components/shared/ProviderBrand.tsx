import type { CSSProperties } from "react";
import { providerMeta, type ProviderId, type ProviderIdentity } from "../../lib/provider";

export interface ProviderBrandProps {
  identity: ProviderIdentity;
  markClassName?: string;
  nameClassName?: string;
}

export interface ProviderBrandPartProps {
  identity: ProviderIdentity;
  className?: string;
}

export function getProviderIdentity(provider: ProviderId): ProviderIdentity {
  return providerMeta[provider];
}

export function providerBrandStyle(identity: ProviderIdentity): CSSProperties {
  return { "--provider-accent": identity.accent } as CSSProperties;
}

export function ProviderBrandDot({
  identity,
  className = "provider-dot",
}: ProviderBrandPartProps) {
  return (
    <span
      className={className}
      style={providerBrandStyle(identity)}
      data-logo-variant={identity.logoVariant}
      data-font-role={identity.fontRole}
      aria-hidden="true"
    >
      <img src={identity.logoSrc} alt="" />
    </span>
  );
}

export function ProviderBrandName({
  identity,
  className,
}: ProviderBrandPartProps) {
  const resolvedClassName =
    className ?? `provider-name provider-name--font-${identity.fontRole}`;
  return (
    <span
      className={resolvedClassName}
      data-logo-variant={identity.logoVariant}
      data-font-role={identity.fontRole}
    >
      {identity.displayName}
    </span>
  );
}

export default function ProviderBrand({
  identity,
  markClassName,
  nameClassName,
}: ProviderBrandProps) {
  return (
    <span
      className="provider-brand"
      data-logo-variant={identity.logoVariant}
      data-font-role={identity.fontRole}
      style={{
        ...providerBrandStyle(identity),
        display: "inline-flex",
        alignItems: "center",
        minWidth: 0,
      }}
    >
      <ProviderBrandDot identity={identity} className={markClassName} />
      <ProviderBrandName identity={identity} className={nameClassName} />
    </span>
  );
}
