import SettingsSwitch from "./SettingsSwitch";
import ThemeSelect from "./ThemeSelect";
import type { ThemeId } from "../../lib/theme";
import formStyles from "../../styles/settings/forms.module.css";
import surfaceStyles from "../../styles/settings/surface.module.css";

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
    <section className={`${surfaceStyles.section} ${surfaceStyles.appearanceSection}`}>
      <div className={surfaceStyles.sectionHeading}>
        <h2 className={surfaceStyles.sectionTitle}>Appearance</h2>
      </div>
      <div className={surfaceStyles.card}>
        <div className={`${surfaceStyles.row} ${formStyles.appearanceRow}`}>
          <span className={formStyles.appearanceLabel} id="theme-label">
            Theme
          </span>
          <ThemeSelect theme={theme} onThemeChange={onThemeChange} />
        </div>
        <div className={`${surfaceStyles.row} ${formStyles.appearanceRow}`}>
          <strong className={formStyles.appearanceLabel}>Dark mode</strong>
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
