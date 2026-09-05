import useSettingsActivity from "../../hooks/useSettingsActivity";
import ProviderVisibilitySection from "./ProviderVisibilitySection";
import SourceSettingsSection from "./SourceSettingsSection";
import type { ProviderId } from "../../lib/provider";
import type { SourcePlatform } from "../../lib/source-settings";
import type {
  SourceFormValues,
  VisibilityValues,
} from "./settings-model";

interface SettingsActivityPanelProps {
  visible: VisibilityValues;
  onProviderVisibilityToggle: (provider: ProviderId, visible: boolean) => void;
  sources: SourceFormValues;
  onSourceToggle: (provider: ProviderId, enabled: boolean) => void;
  onSourceRootChoose: (provider: ProviderId, platform: SourcePlatform) => void | Promise<void>;
  onSourceRootChange: (provider: ProviderId, platform: SourcePlatform, root: string) => void;
  onSourceRootClear: (provider: ProviderId, platform: SourcePlatform) => void;
}

export default function SettingsActivityPanel({
  visible,
  onProviderVisibilityToggle,
  sources,
  onSourceToggle,
  onSourceRootChoose,
  onSourceRootChange,
  onSourceRootClear,
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
        onChangeRoot={onSourceRootChange}
        onClearRoot={onSourceRootClear}
      />
    </>
  );
}
