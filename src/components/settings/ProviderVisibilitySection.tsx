import { providerMeta, providerRegistry, type ProviderId } from "../../lib/provider";
import ProviderDot from "../shared/ProviderDot";
import ProviderName from "../shared/ProviderName";
import SettingsSwitch from "./SettingsSwitch";
import type { ProviderStatusView } from "./settings-types";

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
    <section className="settings-section settings-section--providers">
      <div className="settings-section__heading">
        <h2>Visible providers</h2>
      </div>
      <div className="settings-card">
        {providerRegistry.map(({ id: provider }) => {
          const status = providers.find((entry) => entry.provider === provider);
          const state = status?.state ?? "unavailable";
          const updated = status?.updated ?? "No updates yet";
          return (
            <div className="settings-row provider-settings-row" key={provider}>
              <div className="settings-row__identity">
                <ProviderDot provider={provider} />
                <div>
                  <strong><ProviderName provider={provider} /></strong>
                  <span>
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
