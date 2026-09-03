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

export default function ProviderSection({
  identity,
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
      </div>
      {children}
    </article>
  );
}
