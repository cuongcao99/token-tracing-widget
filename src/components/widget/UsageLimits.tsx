import type { CSSProperties } from "react";
import type { RateLimitSummary } from "../../lib/contracts/usage-summary";
import styles from "../../styles/widget/limits.module.css";

const WINDOW_LABELS: Record<number, string> = {
  300: "5h",
  10_080: "7d",
};
const WINDOW_ORDER = [300, 10_080] as const;

export function limitColor(remainingPercent: number): string {
  const percent = Math.min(100, Math.max(0, remainingPercent));
  const ratio = percent / 100;
  const hue = Math.round(ratio * 134);
  const saturation = Math.round(65 - ratio * 26);
  const lightness = Math.round(50 + ratio * 4);
  return `hsl(${hue} ${saturation}% ${lightness}%)`;
}

export function formatLimitReset(resetsAt: number, nowMs = Date.now()): string {
  const remainingMs = resetsAt * 1000 - nowMs;
  if (remainingMs <= 0) return "Resets now";

  const minutes = Math.ceil(remainingMs / 60_000);
  if (minutes < 60) return `Resets in ${minutes}m`;

  const hours = Math.floor(minutes / 60);
  const remainingMinutes = minutes % 60;
  if (hours < 24) return `Resets in ${hours}h ${remainingMinutes}m`;

  return `Resets ${new Intl.DateTimeFormat("en-US", {
    weekday: "short",
    hour: "numeric",
    minute: "2-digit",
  }).format(new Date(resetsAt * 1000))}`;
}

interface UsageLimitsProps {
  limits: readonly RateLimitSummary[];
}

export default function UsageLimits({ limits }: UsageLimitsProps) {
  const visibleLimits = WINDOW_ORDER.map((windowMinutes) =>
    limits.find((limit) => limit.windowMinutes === windowMinutes),
  ).filter((limit): limit is RateLimitSummary => Boolean(limit));

  if (visibleLimits.length === 0) return null;

  return (
    <div className={styles.root} aria-label="Usage limits">
      {visibleLimits.map((limit) => {
        const label = WINDOW_LABELS[limit.windowMinutes];
        const remainingPercent = 100 - limit.usedPercent;
        const style = {
          "--limit-value": `${remainingPercent}%`,
          "--limit-color": limitColor(remainingPercent),
        } as CSSProperties;

        return (
          <div className={styles.limit} key={limit.windowMinutes} style={style}>
            <div className={styles.header}>
              <span className={styles.label}>{label}</span>
              <span className={styles.value}>{remainingPercent}%</span>
            </div>
            <div
              className={styles.track}
              role="progressbar"
              aria-label={`${label}: ${remainingPercent}% remaining`}
              aria-valuemin={0}
              aria-valuemax={100}
              aria-valuenow={remainingPercent}
            >
              <span className={styles.fill} />
            </div>
            <span className={styles.reset}>
              {formatLimitReset(limit.resetsAt)}
            </span>
          </div>
        );
      })}
    </div>
  );
}
