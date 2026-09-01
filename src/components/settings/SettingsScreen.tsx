import AppearanceSection from "./AppearanceSection";
import SettingsActivityPanel from "./SettingsActivityPanel";
import SettingsCloseButton from "./SettingsCloseButton";
import WindowGrip from "../shared/WindowGrip";
import WindowResizeHandles from "../shared/WindowResizeHandles";
import useSettingsController from "../../hooks/useSettingsController";
import surfaceStyles from "../../styles/settings/surface.module.css";
import formStyles from "../../styles/settings/forms.module.css";

export default function SettingsScreen() {
  const {
    closeSettings,
    darkMode,
    onThemeToggle,
    error,
    loadingSources,
    onDarkModeToggle,
    onProviderVisibilityToggle,
    onSourceRootChoose,
    onSourceToggle,
    sources,
    theme,
    visible,
    widgetError,
  } = useSettingsController();

  return (
    <main
      className={surfaceStyles.root}
      data-theme={theme}
      data-color-mode={darkMode ? "dark" : "light"}
      aria-label="Settings"
    >
      <WindowGrip windowName="settings" />
      <header className={surfaceStyles.header}>
        <div className={surfaceStyles.heading}>
          <div className={surfaceStyles.titleRow}>
            <h1 className={surfaceStyles.title}>Settings</h1>
          </div>
          <p className={surfaceStyles.subtitle}>Choose what stays visible.</p>
        </div>
        <SettingsCloseButton onClick={closeSettings} />
      </header>

      <div className={surfaceStyles.body}>
        {widgetError && (
          <p className={formStyles.status} role="status">
            {widgetError}
          </p>
        )}
        {error && (
          <p className={`${formStyles.status} ${formStyles.statusError}`} role="alert">
            {error}
          </p>
        )}
        {loadingSources && (
          <p className={formStyles.status} role="status">
            Loading settings…
          </p>
        )}

        {sources && (
          <div className={surfaceStyles.form}>
            <SettingsActivityPanel
              visible={visible}
              onProviderVisibilityToggle={onProviderVisibilityToggle}
              sources={sources}
              onSourceToggle={onSourceToggle}
              onSourceRootChoose={onSourceRootChoose}
            />
            <AppearanceSection
              theme={theme}
              onThemeChange={onThemeToggle}
              darkMode={darkMode}
              onToggle={onDarkModeToggle}
            />
          </div>
        )}
      </div>
      <WindowResizeHandles windowName="settings" />
    </main>
  );
}
