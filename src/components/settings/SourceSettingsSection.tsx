import { providerMeta, providerOrder, type ProviderId } from "../../lib/provider";
import type { SourceSettings } from "../../lib/source-settings";
import type { SourceHealth } from "../../lib/usage-summary";
import ProviderDot from "../shared/ProviderDot";
import SettingsSwitch from "./SettingsSwitch";
import { sourceHealthLabel } from "./settings-types";

interface SourceSettingsSectionProps {
  sources: Record<ProviderId, SourceSettings>;
  health: SourceHealth[];
  expanded: Record<ProviderId, boolean>;
  onToggle: (provider: ProviderId, enabled: boolean) => void;
  onRootChange: (provider: ProviderId, rootOverride: string) => void;
  onToggleRoot: (provider: ProviderId) => void;
}

export default function SourceSettingsSection({
  sources,
  health,
  expanded,
  onToggle,
  onRootChange,
  onToggleRoot,
}: SourceSettingsSectionProps) {
  return (
    <section className="settings-section settings-section--sources">
      <div className="settings-section__heading">
        <h2>Sources</h2>
      </div>
      <div className="settings-card">
        {providerOrder.map((provider) => {
          const source = sources[provider];
          const root = source.rootOverride || providerMeta[provider].automaticRoot;
          const isExpanded = expanded[provider];
          return (
            <div className="source-settings-row" key={provider}>
              <div className="settings-row source-settings-row__main">
                <div className="settings-row__identity">
                  <ProviderDot provider={provider} />
                  <div>
                    <strong>{providerMeta[provider].name}</strong>
                    <span>{root}</span>
                  </div>
                </div>
                <div className="source-settings-row__actions">
                  <span className="source-health">
                    <span className="source-health__dot" aria-hidden="true" />
                    {sourceHealthLabel(provider, health, source.enabled)}
                  </span>
                  <SettingsSwitch
                    label={`Collect ${providerMeta[provider].name} source`}
                    checked={source.enabled}
                    onChange={(next) => onToggle(provider, next)}
                  />
                  <button
                    className="change-root-button"
                    type="button"
                    aria-expanded={isExpanded}
                    aria-controls={`${provider}-source-root`}
                    onClick={() => onToggleRoot(provider)}
                  >
                    Change…
                  </button>
                </div>
              </div>
              {isExpanded && (
                <label className="source-root-field" htmlFor={`${provider}-source-root`}>
                  <span>Source root</span>
                  <input
                    id={`${provider}-source-root`}
                    type="text"
                    aria-label={`${providerMeta[provider].name} source root`}
                    value={source.rootOverride ?? ""}
                    placeholder={providerMeta[provider].automaticRoot}
                    onChange={(event) => onRootChange(provider, event.target.value)}
                  />
                </label>
              )}
            </div>
          );
        })}
      </div>
    </section>
  );
}
