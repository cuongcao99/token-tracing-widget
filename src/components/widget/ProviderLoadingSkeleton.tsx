import type { ProviderIdentity } from "../../lib/provider";
import limitsStyles from "../../styles/widget/limits.module.css";
import loadingStyles from "../../styles/widget/loading.module.css";
import metricsStyles from "../../styles/widget/metrics.module.css";
import ProviderSection from "./ProviderSection";

export interface ProviderSkeletonLayout {
  limitSlots: number;
  metricSlots: number;
}

export const DEFAULT_PROVIDER_SKELETON_LAYOUT: ProviderSkeletonLayout = {
  limitSlots: 2,
  metricSlots: 2,
};

export interface ProviderLoadingSkeletonProps {
  identity: ProviderIdentity;
  layout?: ProviderSkeletonLayout;
}

function SkeletonBlock({ className }: { className: string }) {
  return <span className={`${loadingStyles.block} ${className}`} />;
}

function LimitsSkeleton({ count }: { count: number }) {
  if (count <= 0) return null;

  return (
    <div className={limitsStyles.root} aria-hidden="true">
      {Array.from({ length: count }, (_, index) => (
        <div
          className={limitsStyles.limit}
          data-testid="widget-skeleton-limit"
          key={index}
        >
          <div className={limitsStyles.header}>
            <SkeletonBlock className={loadingStyles.limitLabel} />
            <SkeletonBlock className={loadingStyles.limitValue} />
          </div>
          <div className={limitsStyles.track}>
            <SkeletonBlock className={loadingStyles.trackFill} />
          </div>
          <SkeletonBlock className={loadingStyles.limitReset} />
        </div>
      ))}
    </div>
  );
}

function MetricsSkeleton({ count }: { count: number }) {
  return (
    <div className={metricsStyles.metrics} aria-hidden="true">
      {Array.from({ length: count }, (_, index) => (
        <div
          className={metricsStyles.metric}
          data-testid="widget-skeleton-metric"
          key={index}
        >
          <SkeletonBlock className={loadingStyles.metricLabel} />
          <SkeletonBlock className={loadingStyles.metricValue} />
        </div>
      ))}
      <SkeletonBlock
        className={`${metricsStyles.updated} ${loadingStyles.updated}`}
      />
    </div>
  );
}

export default function ProviderLoadingSkeleton({
  identity,
  layout = DEFAULT_PROVIDER_SKELETON_LAYOUT,
}: ProviderLoadingSkeletonProps) {
  return (
    <ProviderSection
      identity={identity}
      status={{ state: "loading", label: "Loading" }}
    >
      <div
        className={loadingStyles.provider}
        data-testid="widget-skeleton-provider"
        aria-hidden="true"
      >
        <LimitsSkeleton count={layout.limitSlots} />
        <MetricsSkeleton count={layout.metricSlots} />
      </div>
    </ProviderSection>
  );
}
