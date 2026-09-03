import { providerMeta, providerRegistry, type ProviderId } from "../../lib/provider";
import type { SourceSettings } from "../../lib/source-settings";
import type { SourceHealth } from "../../lib/usage-summary";
import ProviderDot from "../shared/ProviderDot";
import ProviderName from "../shared/ProviderName";
import SettingsSwitch from "./SettingsSwitch";
import { sourceHealthLabel, sourceHealthState } from "./settings-types";
import formStyles from "../../styles/settings/forms.module.css";
import surfaceStyles from "../../styles/settings/surface.module.css";

interface SourceSettingsSectionProps {
  sources: Record<ProviderId, SourceSettings>;
  health: SourceHealth[];
  onToggle: (provider: ProviderId, enabled: boolean) => void;
  onChooseRoot: (provider: ProviderId) => void | Promise<void>;
}

export default function SourceSettingsSection({
  sources,
  health,
  onToggle,
  onChooseRoot,
}: SourceSettingsSectionProps) {
  return (
    <section className={surfaceStyles.section}>
      <div className={surfaceStyles.sectionHeading}>
        <div>
          <h2 className={surfaceStyles.sectionTitle}>Sources</h2>
          <p className={surfaceStyles.sectionHint}>Collect data from</p>
        </div>
      </div>
      <div className={surfaceStyles.card}>
        {providerRegistry.map(({ id: provider }) => {
          const source = sources[provider];
          const root = source.rootOverride || providerMeta[provider].displayRoot;
          const healthState = sourceHealthState(provider, health, source.enabled);
          return (
            <div className={formStyles.sourceRow} key={provider}>
              <div className={`${surfaceStyles.row} ${formStyles.sourceMain}`}>
                <div className={surfaceStyles.identity}>
                  <ProviderDot provider={provider} />
                  <div className={surfaceStyles.identityContent}>
                    <strong><ProviderName provider={provider} /></strong>
                    <button
                      className={formStyles.sourcePath}
                      type="button"
                      title={root}
                      aria-label={`Choose ${providerMeta[provider].displayName} source folder: ${root}`}
                      onClick={() => void onChooseRoot(provider)}
                    >
                      {root}
                    </button>
                  </div>
                </div>
                <div className={formStyles.sourceActions}>
                  <span
                    className={formStyles.sourceHealth}
                    data-health-state={healthState}
                  >
                    <span className={formStyles.sourceHealthDot} aria-hidden="true" />
                    {sourceHealthLabel(provider, health, source.enabled)}
                  </span>
                  <SettingsSwitch
                    label={`Collect data from ${providerMeta[provider].displayName}`}
                    checked={source.enabled}
                    onChange={(next) => onToggle(provider, next)}
                  />
                </div>
              </div>
            </div>
          );
        })}
      </div>
    </section>
  );
}
