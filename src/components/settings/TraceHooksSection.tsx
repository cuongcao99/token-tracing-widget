import { providerMeta, providerRegistry, type ProviderId } from "../../lib/provider";
import type { TraceHookStatus } from "../../lib/trace-hooks";
import { errorMessage } from "../../lib/settings-model";
import ProviderDot from "../shared/ProviderDot";
import ProviderName from "../shared/ProviderName";
import SettingsSwitch from "./SettingsSwitch";
import surfaceStyles from "../../styles/settings/surface.module.css";
import styles from "../../styles/settings/trace-hooks.module.css";

interface TraceHooksSectionProps {
  statuses: TraceHookStatus[];
  loading: boolean;
  error: Error | null;
  updatingProvider: ProviderId | null;
  onToggle: (provider: ProviderId, enabled: boolean) => void | Promise<void>;
}

function statusLabel(status: TraceHookStatus | undefined): string {
  if (!status || status.state === "not_installed") return "Not installed";
  return status.requiresTrust ? "Configured · review /hooks" : "Installed";
}

export default function TraceHooksSection({
  statuses,
  loading,
  error,
  updatingProvider,
  onToggle,
}: TraceHooksSectionProps) {
  return (
    <section className={styles.section}>
      <div className={surfaceStyles.sectionHeading}>
        <div>
          <h2 className={surfaceStyles.sectionTitle}>Agent tracing</h2>
          <p className={styles.description}>
            Show lightweight live activity from your coding agents.
          </p>
        </div>
      </div>

      {loading && (
        <p className={styles.status} role="status">
          Loading live tracing…
        </p>
      )}
      {error && (
        <p className={styles.statusError} role="alert">
          {errorMessage(error)}
        </p>
      )}

      <div className={surfaceStyles.card}>
        {providerRegistry.map(({ id: provider }) => {
          const status = statuses.find((entry) => entry.provider === provider);
          const configured = status?.state === "configured";
          const disabled = loading || updatingProvider === provider || !status;
          return (
            <div className={surfaceStyles.row} key={provider}>
              <div className={surfaceStyles.identity}>
                <ProviderDot provider={provider} />
                <div className={surfaceStyles.identityContent}>
                  <strong><ProviderName provider={provider} /></strong>
                  <span className={surfaceStyles.identityMeta}>
                    {statusLabel(status)}
                  </span>
                </div>
              </div>
              <SettingsSwitch
                label={`Configure ${providerMeta[provider].displayName} live tracing hook`}
                checked={configured}
                disabled={disabled}
                onChange={(next) => void onToggle(provider, next)}
              />
            </div>
          );
        })}
      </div>

      <p className={styles.note}>
        Hooks send lifecycle hints only. Token totals still come from session files.
      </p>
      {statuses.some((status) => status.provider === "codex" && status.requiresTrust) && (
        <p className={styles.trustNote}>
          Codex trust is managed by Codex. Review the hook in /hooks before it can run.
        </p>
      )}
    </section>
  );
}
