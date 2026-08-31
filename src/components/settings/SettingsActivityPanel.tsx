import useSettingsActivity from "../../hooks/useSettingsActivity";
import ProviderVisibilitySection from "./ProviderVisibilitySection";
import SourceSettingsSection from "./SourceSettingsSection";
import type { ProviderId } from "../../lib/provider";
import type {
  SourceFormValues,
  VisibilityValues,
} from "./settings-model";

interface SettingsActivityPanelProps {
  visible: VisibilityValues;
  onProviderVisibilityToggle: (provider: ProviderId, visible: boolean) => void;
  sources: SourceFormValues;
  onSourceToggle: (provider: ProviderId, enabled: boolean) => void;
  onSourceRootChoose: (provider: ProviderId) => void | Promise<void>;
}

export default function SettingsActivityPanel({
  visible,
  onProviderVisibilityToggle,
  sources,
  onSourceToggle,
  onSourceRootChoose,
}: SettingsActivityPanelProps) {
  const { summary, providerStatuses } = useSettingsActivity();

  return (
    <>
      <ProviderVisibilitySection
        visible={visible}
        providers={providerStatuses}
        onToggle={onProviderVisibilityToggle}
      />
      <SourceSettingsSection
        sources={sources}
        health={summary.sourceHealth}
        onToggle={onSourceToggle}
        onChooseRoot={onSourceRootChoose}
      />
    </>
  );
}
