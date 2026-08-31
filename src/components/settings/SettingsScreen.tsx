import AppearanceSection from "./AppearanceSection";
import ProviderVisibilitySection from "./ProviderVisibilitySection";
import SettingsCloseButton from "./SettingsCloseButton";
import SourceSettingsSection from "./SourceSettingsSection";
import WindowGrip from "../shared/WindowGrip";
import WindowResizeHandles from "../shared/WindowResizeHandles";
import useSettingsController from "../../hooks/useSettingsController";

export default function SettingsScreen() {
  const {
    closeSettings,
    darkMode,
    onThemeToggle,
    error,
    expanded,
    loadingSources,
    onDarkModeToggle,
    onProviderVisibilityToggle,
    onSourceRootBlur,
    onSourceRootChange,
    onSourceRootToggle,
    onSourceToggle,
    providerStatuses,
    sources,
    summary,
    theme,
    visible,
    widgetError,
  } = useSettingsController();

  return (
    <main
      className={`settings-page theme--${theme} settings-page--${darkMode ? "dark" : "light"}`}
      aria-label="Settings"
    >
      <WindowGrip windowName="settings" />
      <header className="settings-page__header">
        <div className="settings-page__heading">
          <div className="settings-page__title-row">
            <h1>Settings</h1>
          </div>
          <p>Choose what stays visible.</p>
        </div>
        <SettingsCloseButton onClick={closeSettings} />
      </header>

      <div className="settings-page__body">
        {widgetError && (
          <p className="settings-status" role="status">
            {widgetError}
          </p>
        )}
        {error && (
          <p className="settings-status settings-status--error" role="alert">
            {error}
          </p>
        )}
        {loadingSources && (
          <p className="settings-status" role="status">
            Loading settings…
          </p>
        )}

        {sources && (
          <div className="settings-form">
            <ProviderVisibilitySection
              visible={visible}
              providers={providerStatuses}
              onToggle={onProviderVisibilityToggle}
            />
            <SourceSettingsSection
              sources={sources}
              health={summary.sourceHealth}
              expanded={expanded}
              onToggle={onSourceToggle}
              onRootChange={onSourceRootChange}
              onRootBlur={onSourceRootBlur}
              onToggleRoot={onSourceRootToggle}
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
