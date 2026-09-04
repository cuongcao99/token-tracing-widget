import { useEffect, useRef, useState } from "react";
import {
  providerMeta,
  providerRegistry,
  type ProviderId,
} from "../../lib/provider";
import type { SourcePlatform, SourceSettings } from "../../lib/source-settings";
import formStyles from "../../styles/settings/forms.module.css";
import ProviderDot from "../shared/ProviderDot";
import ProviderName from "../shared/ProviderName";

interface SourcePickerDialogProps {
  sources: Record<ProviderId, SourceSettings>;
  onChooseRoot: (provider: ProviderId, platform: SourcePlatform) => void | Promise<void>;
  onChangeRoot: (provider: ProviderId, platform: SourcePlatform, root: string) => void;
  onClearRoot: (provider: ProviderId, platform: SourcePlatform) => void;
  onClose: () => void;
}

type DraftRoots = Record<ProviderId, Record<SourcePlatform, string>>;

function draftsFromSources(sources: Record<ProviderId, SourceSettings>): DraftRoots {
  const drafts = {} as DraftRoots;
  for (const { id: provider } of providerRegistry) {
    drafts[provider] = {
      windows: sources[provider].windowsRoot ?? "",
      wsl: sources[provider].wslRoot ?? "",
    };
  }
  return drafts;
}

function placeholderFor(provider: ProviderId, platform: SourcePlatform) {
  if (platform === "windows") return providerMeta[provider].displayRoot;
  return `\\\\wsl.localhost\\<distribution>\\home\\<user>\\${providerMeta[
    provider
  ].displayRoot.replace(/^~[\\/]/, "").replace("/", "\\")}`;
}

export default function SourcePickerDialog({
  sources,
  onChooseRoot,
  onChangeRoot,
  onClearRoot,
  onClose,
}: SourcePickerDialogProps) {
  const closeButton = useRef<HTMLButtonElement>(null);
  const [drafts, setDrafts] = useState<DraftRoots>(() => draftsFromSources(sources));

  useEffect(() => {
    setDrafts(draftsFromSources(sources));
  }, [sources]);

  useEffect(() => {
    closeButton.current?.focus();
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") onClose();
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [onClose]);

  const commitRoot = (provider: ProviderId, platform: SourcePlatform) => {
    const value = drafts[provider][platform].trim();
    const source = sources[provider];
    const currentRoot = platform === "windows" ? source.windowsRoot : source.wslRoot;
    if (value === (currentRoot ?? "")) return;
    if (value) {
      onChangeRoot(provider, platform, value);
    } else {
      onClearRoot(provider, platform);
    }
  };

  const renderOption = (
    provider: ProviderId,
    platform: SourcePlatform,
    label: string,
    configured: boolean,
  ) => (
    <div className={formStyles.sourceDialogOption} key={platform}>
      <div className={formStyles.sourceDialogOptionHeading}>
        <h4>{label}</h4>
        <span>
          {configured ? "Configured" : platform === "wsl" ? "Not configured" : "Automatic"}
        </span>
      </div>
      <div className={formStyles.sourceDialogPathControl}>
        <input
          className={`${formStyles.sourceDialogPath} ${
            drafts[provider][platform]
              ? formStyles.sourceDialogPathConfigured
              : formStyles.sourceDialogPathPlaceholder
          }`}
          type="text"
          value={drafts[provider][platform]}
          placeholder={placeholderFor(provider, platform)}
          aria-label={`Edit ${providerMeta[provider].displayName} ${label} source folder`}
          autoComplete="off"
          spellCheck={false}
          onChange={(event) =>
            setDrafts((current) => ({
              ...current,
              [provider]: { ...current[provider], [platform]: event.target.value },
            }))
          }
          onBlur={() => commitRoot(provider, platform)}
          onKeyDown={(event) => {
            if (event.key === "Enter") {
              event.preventDefault();
              event.currentTarget.blur();
            }
          }}
        />
        <button
          className={formStyles.sourceDialogBrowse}
          type="button"
          aria-label={`Browse ${providerMeta[provider].displayName} ${label} source folder`}
          title="Browse path"
          onClick={() => void onChooseRoot(provider, platform)}
        >
          <svg
            aria-hidden="true"
            fill="none"
            stroke="currentColor"
            strokeLinecap="round"
            strokeLinejoin="round"
            strokeWidth="1.7"
            viewBox="0 0 24 24"
          >
            <path d="M3.75 7.5A1.75 1.75 0 0 1 5.5 5.75h3.4l1.7 2h7.9a1.75 1.75 0 0 1 1.75 1.75v7.25a1.5 1.5 0 0 1-1.5 1.5h-13.5a1.5 1.5 0 0 1-1.5-1.5z" />
            <path d="M3.75 9.25h16.5" />
          </svg>
        </button>
      </div>
      <div className={formStyles.sourceDialogActions}>
        {platform === "windows" && configured && (
          <button
            className={formStyles.sourceDialogSecondary}
            type="button"
            onClick={() => onClearRoot(provider, platform)}
          >
            Use Windows default
          </button>
        )}
        {platform === "wsl" && configured && (
          <button
            className={formStyles.sourceDialogSecondary}
            type="button"
            onClick={() => onClearRoot(provider, platform)}
          >
            Remove WSL source
          </button>
        )}
      </div>
    </div>
  );

  const renderProvider = (provider: ProviderId) => (
    <section
      className={formStyles.sourceDialogProvider}
      key={provider}
      aria-labelledby={`source-dialog-${provider}`}
    >
      <div className={formStyles.sourceDialogProviderHeading}>
        <ProviderDot provider={provider} />
        <h3 id={`source-dialog-${provider}`}><ProviderName provider={provider} /></h3>
      </div>
      <div className={formStyles.sourceDialogProviderOptions}>
        {renderOption(provider, "windows", "Windows", Boolean(sources[provider].windowsRoot))}
        {renderOption(provider, "wsl", "WSL", Boolean(sources[provider].wslRoot))}
      </div>
    </section>
  );

  return (
    <div className={formStyles.sourceDialogBackdrop}>
      <section
        className={formStyles.sourceDialog}
        role="dialog"
        aria-modal="true"
        aria-labelledby="source-dialog-title"
      >
        <div className={formStyles.sourceDialogHeader}>
          <div>
            <h2 id="source-dialog-title" className={formStyles.sourceDialogAccentTitle}>
              Change source
            </h2>
          </div>
          <button
            ref={closeButton}
            className={formStyles.sourceDialogClose}
            type="button"
            aria-label="Close source chooser"
            onClick={onClose}
          >
            ×
          </button>
        </div>
        <p className={formStyles.sourceDialogHint}>
          Windows and WSL can both be collected at the same time.
        </p>
        <div className={formStyles.sourceDialogProviders}>
          {providerRegistry.map(({ id: provider }) => renderProvider(provider))}
        </div>
      </section>
    </div>
  );
}
