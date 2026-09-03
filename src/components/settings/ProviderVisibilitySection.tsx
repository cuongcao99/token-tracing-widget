import { providerMeta, providerRegistry, type ProviderId } from "../../lib/provider";
import ProviderDot from "../shared/ProviderDot";
import ProviderName from "../shared/ProviderName";
import SettingsSwitch from "./SettingsSwitch";
import {
  providerActivityLabel,
  type ProviderStatusView,
} from "./settings-types";
import surfaceStyles from "../../styles/settings/surface.module.css";

interface ProviderVisibilitySectionProps {
  visible: Record<ProviderId, boolean>;
  providers: ProviderStatusView[];
  onToggle: (provider: ProviderId, visible: boolean) => void;
}

export default function ProviderVisibilitySection({
  visible,
  providers,
  onToggle,
}: ProviderVisibilitySectionProps) {
  return (
    <section className={surfaceStyles.section}>
      <div className={surfaceStyles.sectionHeading}>
        <div>
          <h2 className={surfaceStyles.sectionTitle}>Visible providers</h2>
          <p className={surfaceStyles.sectionHint}>Show in widget</p>
        </div>
      </div>
      <div className={surfaceStyles.card}>
        {providerRegistry.map(({ id: provider }) => {
          const status = providers.find((entry) => entry.provider === provider);
          const state = providerActivityLabel(status?.state ?? "unavailable");
          const updated = status?.updated ?? "No updates yet";
          return (
            <div className={surfaceStyles.row} key={provider}>
              <div className={surfaceStyles.identity}>
                <ProviderDot provider={provider} />
                <div className={surfaceStyles.identityContent}>
                  <strong><ProviderName provider={provider} /></strong>
                  <span className={surfaceStyles.identityMeta}>
                    {state.charAt(0).toUpperCase() + state.slice(1)} · {updated}
                  </span>
                </div>
              </div>
              <SettingsSwitch
                label={`Show ${providerMeta[provider].displayName} in widget`}
                checked={visible[provider]}
                onChange={(next) => onToggle(provider, next)}
              />
            </div>
          );
        })}
      </div>
    </section>
  );
}
