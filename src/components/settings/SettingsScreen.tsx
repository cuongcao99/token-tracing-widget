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
    error,
    expanded,
    handleWindowMouseDown,
    loadingSources,
    onDarkModeToggle,
    onProviderVisibilityToggle,
    onSourceRootChange,
    onSourceRootToggle,
    onSourceToggle,
    providerStatuses,
    save,
    saved,
    saving,
    sources,
    summary,
    visible,
    widgetError,
  } = useSettingsController();

  return (
    <main
      className={`settings-page settings-page--${darkMode ? "dark" : "light"}`}
      aria-label="Settings"
    >
      <header
        className="settings-page__header"
        onMouseDown={handleWindowMouseDown}
      >
        <div className="settings-page__heading" data-tauri-drag-region="">
          <div className="settings-page__title-row">
            <WindowGrip />
            <h1>Settings</h1>
          </div>
          <p>Choose what stays visible.</p>
        </div>
        <SettingsCloseButton onClick={closeSettings} />
      </header>

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
        <form className="settings-form" onSubmit={save}>
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
            onToggleRoot={onSourceRootToggle}
          />
          <AppearanceSection
            darkMode={darkMode}
            onToggle={onDarkModeToggle}
          />
          <div className="settings-actions">
            {saved && <p role="status">Saved.</p>}
            <button className="save-button" type="submit" disabled={saving}>
              {saving ? "Saving…" : "Save changes"}
            </button>
          </div>
        </form>
      )}
      <WindowResizeHandles windowName="settings" />
    </main>
  );
}
