import SettingsSwitch from "./SettingsSwitch";

interface AppearanceSectionProps {
  darkMode: boolean;
  onToggle: (darkMode: boolean) => void;
}

export default function AppearanceSection({
  darkMode,
  onToggle,
}: AppearanceSectionProps) {
  return (
    <section className="settings-section settings-section--appearance">
      <div className="settings-section__heading">
        <h2>Appearance</h2>
      </div>
      <div className="settings-card appearance-card">
        <div className="settings-row appearance-row">
          <strong>Dark mode</strong>
          <SettingsSwitch
            label="Dark mode"
            checked={darkMode}
            onChange={onToggle}
          />
        </div>
      </div>
    </section>
  );
}
