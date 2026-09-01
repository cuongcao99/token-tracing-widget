import type { UsageMetric } from "./widget-types";
import styles from "../../styles/widget/metrics.module.css";

export interface UsageMetricsProps {
  metrics: readonly UsageMetric[];
  updatedLabel: string;
}

export default function UsageMetrics({
  metrics,
  updatedLabel,
}: UsageMetricsProps) {
  return (
    <div className={styles.metrics}>
      {metrics.map((metric, index) => (
        <div className={styles.metric} key={`${metric.label}-${index}`}>
          <span className={styles.label}>{metric.label}</span>
          <strong className={styles.value} aria-label={metric.ariaLabel}>
            {metric.value}
          </strong>
        </div>
      ))}
      <span className={styles.updated}>{updatedLabel}</span>
    </div>
  );
}
