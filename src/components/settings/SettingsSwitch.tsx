interface SettingsSwitchProps {
  label: string;
  checked: boolean;
  onChange: (checked: boolean) => void;
  disabled?: boolean;
}

export default function SettingsSwitch({
  label,
  checked,
  onChange,
  disabled = false,
}: SettingsSwitchProps) {
  return (
    <button
      className={`settings-switch${checked ? " is-on" : ""}`}
      type="button"
      role="switch"
      aria-label={label}
      aria-checked={checked}
      disabled={disabled}
      onClick={() => onChange(!checked)}
    >
      <span className="settings-switch__knob" aria-hidden="true" />
    </button>
  );
}
