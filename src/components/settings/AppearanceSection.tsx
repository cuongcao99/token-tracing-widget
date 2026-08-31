import SettingsSwitch from "./SettingsSwitch";
import ThemeSelect from "./ThemeSelect";
import type { ThemeId } from "../../lib/theme";

interface AppearanceSectionProps {
  theme: ThemeId;
  onThemeChange: (theme: ThemeId) => void;
  darkMode: boolean;
  onToggle: (darkMode: boolean) => void;
}

export default function AppearanceSection({
  theme,
  onThemeChange,
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
          <span id="theme-label">Theme</span>
          <ThemeSelect theme={theme} onThemeChange={onThemeChange} />
        </div>
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
