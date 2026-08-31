import SettingsSwitch from "./SettingsSwitch";
import { themeRegistry, type ThemeId } from "../../lib/theme";

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
          <label htmlFor="theme-select">Theme</label>
          <select
            id="theme-select"
            aria-label="Theme"
            value={theme}
            onChange={(event) => onThemeChange(event.target.value as ThemeId)}
          >
            {themeRegistry.map((option) => (
              <option key={option.id} value={option.id}>
                {option.label}
              </option>
            ))}
          </select>
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
