import type { ReactNode } from "react";
import type { UsageState } from "../../lib/usage-summary";
import type { ProviderIdentity } from "../../lib/provider";
import styles from "../../styles/widget/provider.module.css";
import ProviderBrand from "../shared/ProviderBrand";

export interface ProviderSectionProps {
  identity: ProviderIdentity;
  status: { state: UsageState; label: string };
  children: ReactNode;
  className?: string;
  markClassName?: string;
  nameClassName?: string;
}

const statusStyles: Record<UsageState, string> = {
  loading: styles.statusLoading,
  active: styles.statusActive,
  idle: styles.statusIdle,
  unavailable: styles.statusUnavailable,
  stale: styles.statusStale,
};

export default function ProviderSection({
  identity,
  status,
  children,
  className,
  markClassName,
  nameClassName,
}: ProviderSectionProps) {
  const sectionClassName = [styles.section, className].filter(Boolean).join(" ");
  const resolvedNameClassName = nameClassName ?? "";

  return (
    <article className={sectionClassName}>
      <div className={styles.heading}>
        <h2 className={styles.headingTitle}>
          <ProviderBrand
            identity={identity}
            markClassName={markClassName}
            nameClassName={resolvedNameClassName}
          />
        </h2>
        <span className={`${styles.status} ${statusStyles[status.state]}`}>
          <span className={styles.statusDot} aria-hidden="true" />
          {status.label}
        </span>
      </div>
      {children}
    </article>
  );
}
