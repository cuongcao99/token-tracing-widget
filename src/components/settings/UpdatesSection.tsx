import SettingsSwitch from "./SettingsSwitch";
import useAppUpdates from "../../hooks/useAppUpdates";
import formStyles from "../../styles/settings/forms.module.css";
import surfaceStyles from "../../styles/settings/surface.module.css";

interface UpdatesSectionProps {
  autoUpdate: boolean;
  loadingSettings: boolean;
  onAutoUpdateToggle: (autoUpdate: boolean) => void;
}

function statusText(
  status: ReturnType<typeof useAppUpdates>["status"],
  currentVersion: string | null,
  availableVersion: string | null,
  error: string | null,
): string {
  switch (status) {
    case "checking":
      return "Checking for updates…";
    case "up-to-date":
      return currentVersion ? `You're up to date (v${currentVersion}).` : "You're up to date.";
    case "available":
      return availableVersion
        ? `Version ${availableVersion} is available.`
        : "An update is available.";
    case "installing":
      return "Installing update…";
    case "error":
      return error ?? "Could not check for updates.";
    default:
      return "Check for a signed release.";
  }
}

export default function UpdatesSection({
  autoUpdate,
  loadingSettings,
  onAutoUpdateToggle,
}: UpdatesSectionProps) {
  const update = useAppUpdates();
  const operationActive = update.status === "checking" || update.status === "installing";

  return (
    <section className={`${surfaceStyles.section} ${surfaceStyles.updatesSection}`}>
      <div className={surfaceStyles.sectionHeading}>
        <div>
          <h2 className={surfaceStyles.sectionTitle}>Updates</h2>
          <p className={surfaceStyles.sectionHint}>Keep the app current with signed releases.</p>
        </div>
      </div>
      <div className={surfaceStyles.card}>
        <div className={surfaceStyles.row}>
          <div className={surfaceStyles.identity}>
            <div className={surfaceStyles.identityContent}>
              <strong>Automatic updates</strong>
              <span className={surfaceStyles.identityMeta}>
                Check and install once when the app starts.
              </span>
            </div>
          </div>
          <SettingsSwitch
            label="Automatic updates"
            checked={autoUpdate}
            disabled={loadingSettings}
            onChange={onAutoUpdateToggle}
          />
        </div>
        <div className={`${surfaceStyles.row} ${formStyles.updateRow}`}>
          <div className={surfaceStyles.identity}>
            <div className={surfaceStyles.identityContent}>
              <strong>Application version</strong>
              <span className={surfaceStyles.identityMeta} role="status" aria-live="polite">
                {statusText(
                  update.status,
                  update.currentVersion,
                  update.availableVersion,
                  update.error,
                )}
              </span>
            </div>
          </div>
          <button
            className={formStyles.sourceManageButton}
            type="button"
            disabled={operationActive}
            onClick={() => void update.checkForUpdates()}
          >
            {update.status === "checking" ? "Checking…" : "Check for updates"}
          </button>
        </div>
        {update.status === "available" && (
          <div className={`${surfaceStyles.row} ${formStyles.updateRow}`}>
            <div className={surfaceStyles.identity}>
              <div className={surfaceStyles.identityContent}>
                <strong>Update ready</strong>
                <span className={surfaceStyles.identityMeta}>
                  Install and restart to apply it.
                </span>
              </div>
            </div>
            <button
              className={formStyles.sourceManageButton}
              type="button"
              onClick={() => void update.installUpdate()}
            >
              Install update
            </button>
          </div>
        )}
      </div>
    </section>
  );
}
