import { providerMeta, providerRegistry, type ProviderId } from "../../lib/provider";
import type { SourceSettings } from "../../lib/source-settings";
import type { SourceHealth } from "../../lib/usage-summary";
import ProviderDot from "../shared/ProviderDot";
import ProviderName from "../shared/ProviderName";
import SettingsSwitch from "./SettingsSwitch";
import { sourceHealthLabel } from "./settings-types";

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
    <section className="settings-section settings-section--sources">
      <div className="settings-section__heading">
        <h2>Sources</h2>
      </div>
      <div className="settings-card">
        {providerRegistry.map(({ id: provider }) => {
          const source = sources[provider];
          const root = source.rootOverride || providerMeta[provider].displayRoot;
          return (
            <div className="source-settings-row" key={provider}>
              <div className="settings-row source-settings-row__main">
                <div className="settings-row__identity">
                  <ProviderDot provider={provider} />
                  <div>
                    <strong><ProviderName provider={provider} /></strong>
                    <button
                      className="source-path-button"
                      type="button"
                      title={root}
                      aria-label={`Choose ${providerMeta[provider].displayName} source folder: ${root}`}
                      onClick={() => void onChooseRoot(provider)}
                    >
                      {root}
                    </button>
                  </div>
                </div>
                <div className="source-settings-row__actions">
                  <span className="source-health">
                    <span className="source-health__dot" aria-hidden="true" />
                    {sourceHealthLabel(provider, health, source.enabled)}
                  </span>
                  <SettingsSwitch
                    label={`Collect ${providerMeta[provider].displayName} source`}
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
