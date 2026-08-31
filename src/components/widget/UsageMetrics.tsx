import type { UsageMetric } from "./widget-types";

export interface UsageMetricsProps {
  metrics: readonly UsageMetric[];
  updatedLabel: string;
}

export default function UsageMetrics({
  metrics,
  updatedLabel,
}: UsageMetricsProps) {
  return (
    <div className="widget-provider__metrics">
      {metrics.map((metric, index) => (
        <div className="widget-metric" key={`${metric.label}-${index}`}>
          <span>{metric.label}</span>
          <strong aria-label={metric.ariaLabel}>{metric.value}</strong>
        </div>
      ))}
      <span className="widget-provider__updated">{updatedLabel}</span>
    </div>
  );
}
