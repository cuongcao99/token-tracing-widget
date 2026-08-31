import type { ReactNode } from "react";
import type { UsageState } from "../../lib/usage-summary";
import type { ProviderIdentity } from "../../lib/provider";
import ProviderBrand from "../shared/ProviderBrand";

export interface ProviderSectionProps {
  identity: ProviderIdentity;
  status: { state: UsageState; label: string };
  children: ReactNode;
  className?: string;
  markClassName?: string;
  nameClassName?: string;
}

export default function ProviderSection({
  identity,
  status,
  children,
  className = "widget-provider",
  markClassName,
  nameClassName,
}: ProviderSectionProps) {
  return (
    <article className={className}>
      <div className="widget-provider__heading">
        <h2>
          <ProviderBrand
            identity={identity}
            markClassName={markClassName}
            nameClassName={nameClassName}
          />
        </h2>
        <span className={`provider-status provider-status--${status.state}`}>
          <span className="provider-status__dot" aria-hidden="true" />
          {status.label}
        </span>
      </div>
      {children}
    </article>
  );
}
