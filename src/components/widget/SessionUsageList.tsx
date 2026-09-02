import type { WidgetSessionViewModel } from "../../lib/widget-view-model";
import styles from "../../styles/widget/sessions.module.css";
import { formatTokens } from "./widget-types";

export interface SessionUsageListProps {
  sessions: readonly WidgetSessionViewModel[];
  onToggle?: () => void;
}

function shortenSessionId(id: string): string {
  const leadingCharacters = 8;
  const trailingCharacters = 5;
  const ellipsisLength = 1;
  if (id.length <= leadingCharacters + trailingCharacters + ellipsisLength) {
    return id;
  }
  return `${id.slice(0, leadingCharacters)}…${id.slice(-trailingCharacters)}`;
}

function SessionRow({ session }: {
  session: WidgetSessionViewModel;
}) {
  const tokens = formatTokens(session.todayTokens);
  const isFallbackId = session.label === session.id;
  const label = isFallbackId ? shortenSessionId(session.id) : session.label;
  const sessionLabel = isFallbackId ? (
    <button
      type="button"
      className={`${styles.labelButton} ${styles.idLabel}`}
      title={session.id}
      aria-label={`Copy session ID ${session.id}`}
      onClick={() => {
        try {
          const clipboard = navigator.clipboard;
          if (clipboard) {
            void clipboard.writeText(session.id).catch(() => undefined);
          }
        } catch {
          // Clipboard access is optional in an embedded webview.
        }
      }}
    >
      <span className={styles.label}>{label}</span>
    </button>
  ) : (
    <span className={styles.label} title={session.label}>
      {label}
    </span>
  );

  return (
    <div
      className={styles.row}
      role="group"
      aria-label={`${session.label}: ${tokens} tokens`}
    >
      {sessionLabel}
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
        <SessionRow
          key={session.id}
          session={session}
        />
      ))}
      {idle.length > 0 && (
        <details className={styles.disclosure} onToggle={onToggle}>
          <summary className={styles.summary}>Idle · {idle.length}</summary>
          <div className={styles.idleRows}>
            {idle.map((session) => (
              <SessionRow
                key={session.id}
                session={session}
              />
            ))}
          </div>
        </details>
      )}
    </div>
  );
}
