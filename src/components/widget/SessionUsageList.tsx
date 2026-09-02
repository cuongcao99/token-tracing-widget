import type { WidgetSessionViewModel } from "../../lib/widget-view-model";
import styles from "../../styles/widget/sessions.module.css";
import { formatTokens } from "./widget-types";

export interface SessionUsageListProps {
  sessions: readonly WidgetSessionViewModel[];
  onToggle?: () => void;
}

function SessionRow({ session }: { session: WidgetSessionViewModel }) {
  const tokens = formatTokens(session.todayTokens);

  return (
    <div
      className={styles.row}
      role="group"
      aria-label={`${session.label}: ${tokens} tokens`}
    >
      <span className={styles.label} title={session.label}>
        {session.label}
      </span>
      <strong className={styles.tokens}>{tokens}</strong>
    </div>
  );
}

export default function SessionUsageList({
  sessions,
  onToggle,
}: SessionUsageListProps) {
  const active = sessions.filter((session) => session.state === "active");
  const idle = sessions.filter((session) => session.state === "idle");

  if (sessions.length === 0) return null;

  return (
    <div className={styles.list} aria-label="Today's sessions">
      {active.map((session) => (
        <SessionRow key={session.id} session={session} />
      ))}
      {idle.length > 0 && (
        <details className={styles.disclosure} onToggle={onToggle}>
          <summary className={styles.summary}>Idle · {idle.length}</summary>
          <div className={styles.idleRows}>
            {idle.map((session) => (
              <SessionRow key={session.id} session={session} />
            ))}
          </div>
        </details>
      )}
    </div>
  );
}
