import { useEffect, useState, type FormEvent } from "react";
import {
  getSourceSettings,
  updateSourceSettings,
  type ProviderId,
  type SourceSettingsSnapshot,
} from "./lib/source-settings";

const providerOrder: ProviderId[] = ["claude", "codex"];
const providerNames: Record<ProviderId, string> = {
  claude: "Claude Code",
  codex: "Codex",
};

interface ProviderFormValues {
  enabled: boolean;
  rootOverride: string;
}

type FormValues = Record<ProviderId, ProviderFormValues>;

function formValuesFromSnapshot(snapshot: SourceSettingsSnapshot): FormValues {
  const values = {} as FormValues;
  for (const provider of providerOrder) {
    const source = snapshot.sources.find((entry) => entry.provider === provider);
    if (!source) {
      throw new Error("invalid_source_settings");
    }
    values[provider] = {
      enabled: source.enabled,
      rootOverride: source.rootOverride ?? "",
    };
  }
  return values;
}

function errorMessage(error: unknown): string {
  const code = error instanceof Error ? error.message : "";
  if (code.startsWith("invalid_root:")) {
    return "Invalid source root. Use an absolute Windows path or an approved WSL path.";
  }
  if (code === "settings_write") {
    return "Could not save source settings.";
  }
  if (code === "settings_refresh") {
    return "Settings were not applied because collection could not refresh.";
  }
  return "Source settings are unavailable.";
}

export default function Settings() {
  const [formValues, setFormValues] = useState<FormValues | null>(null);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [saved, setSaved] = useState(false);

  useEffect(() => {
    let mounted = true;

    const load = async () => {
      try {
        const snapshot = await getSourceSettings();
        if (mounted) {
          setFormValues(formValuesFromSnapshot(snapshot));
          setError(null);
        }
      } catch (loadError) {
        if (mounted) {
          setError(errorMessage(loadError));
        }
      } finally {
        if (mounted) {
          setLoading(false);
        }
      }
    };

    void load();
    return () => {
      mounted = false;
    };
  }, []);

  const updateProvider = (
    provider: ProviderId,
    changes: Partial<ProviderFormValues>,
  ) => {
    setSaved(false);
    setFormValues((current) => {
      if (!current) return current;
      return {
        ...current,
        [provider]: { ...current[provider], ...changes },
      };
    });
  };

  const save = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    if (!formValues) return;

    setSaving(true);
    setSaved(false);
    setError(null);
    try {
      for (const provider of providerOrder) {
        const values = formValues[provider];
        await updateSourceSettings({
          provider,
          enabled: values.enabled,
          rootOverride: values.rootOverride.trim() || null,
        });
      }
      setSaved(true);
    } catch (saveError) {
      setError(errorMessage(saveError));
    } finally {
      setSaving(false);
    }
  };

  return (
    <main className="settings-page" aria-label="Source settings">
      <header className="settings-page__header">
        <div>
          <p className="settings-page__eyebrow">Token Tracing</p>
          <h1>Source settings</h1>
        </div>
        <span className="settings-page__badge">Local only</span>
      </header>

      <p className="settings-page__intro">
        Choose which local provider sources to collect. Leave a root empty to
        use its automatic native path.
      </p>

      {loading && <p role="status">Loading settings…</p>}
      {error && <p role="alert">{error}</p>}

      {formValues && (
        <form className="settings-form" onSubmit={save}>
          {providerOrder.map((provider) => {
            const name = providerNames[provider];
            const values = formValues[provider];
            return (
              <section className="source-card" key={provider}>
                <div className="source-card__header">
                  <div>
                    <h2>{name}</h2>
                    <p>Read token metadata from this source.</p>
                  </div>
                  <label className="source-card__toggle">
                    <input
                      type="checkbox"
                      aria-label={`Collect ${name}`}
                      checked={values.enabled}
                      onChange={(event) =>
                        updateProvider(provider, {
                          enabled: event.target.checked,
                        })
                      }
                    />
                    <span>Collect</span>
                  </label>
                </div>

                <label className="settings-field">
                  <span>Source root</span>
                  <input
                    type="text"
                    aria-label={`${name} source root`}
                    value={values.rootOverride}
                    placeholder="Automatic native path"
                    onChange={(event) =>
                      updateProvider(provider, {
                        rootOverride: event.target.value,
                      })
                    }
                  />
                </label>
                <p className="settings-field__hint">
                  Use an absolute Windows path, or an approved
                  \\wsl.localhost path for WSL.
                </p>
              </section>
            );
          })}

          <div className="settings-form__actions">
            <button type="submit" disabled={saving}>
              {saving ? "Saving…" : "Save changes"}
            </button>
            {saved && (
              <p role="status">Saved. Collection will refresh shortly.</p>
            )}
          </div>
        </form>
      )}
    </main>
  );
}
