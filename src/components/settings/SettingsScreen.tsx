import AppearanceSection from "./AppearanceSection";
import SettingsActivityPanel from "./SettingsActivityPanel";
import SettingsCloseButton from "./SettingsCloseButton";
import UpdatesSection from "./UpdatesSection";
import WindowGrip from "../shared/WindowGrip";
import WindowResizeHandles from "../shared/WindowResizeHandles";
import useSettingsController from "../../hooks/useSettingsController";
import surfaceStyles from "../../styles/settings/surface.module.css";
import formStyles from "../../styles/settings/forms.module.css";

export default function SettingsScreen() {
  const {
    closeSettings,
    darkMode,
    autoUpdate,
    loadingUpdateSettings,
    onThemeToggle,
    onAutoUpdateToggle,
    error,
    loadingSources,
    onDarkModeToggle,
    onProviderVisibilityToggle,
    onSourceRootChoose,
    onSourceRootChange,
    onSourceRootClear,
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
              onSourceRootChange={onSourceRootChange}
              onSourceRootClear={onSourceRootClear}
            />
            <AppearanceSection
              theme={theme}
              onThemeChange={onThemeToggle}
              darkMode={darkMode}
              onToggle={onDarkModeToggle}
            />
            <UpdatesSection
              autoUpdate={autoUpdate}
              loadingSettings={loadingUpdateSettings}
              onAutoUpdateToggle={onAutoUpdateToggle}
            />
          </div>
        )}
      </div>
      <WindowResizeHandles windowName="settings" />
    </main>
  );
}
