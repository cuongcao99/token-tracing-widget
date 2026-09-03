import styles from "../../styles/settings/surface.module.css";

interface SettingsCloseButtonProps {
  onClick: () => void;
}

export default function SettingsCloseButton({ onClick }: SettingsCloseButtonProps) {
  return (
    <button
      className={styles.closeButton}
      type="button"
      aria-label="Close settings"
      onClick={onClick}
    >
      <svg
        className={styles.closeIcon}
        viewBox="0 0 24 24"
        aria-hidden="true"
        fill="none"
        stroke="currentColor"
        strokeLinecap="round"
        strokeLinejoin="round"
        strokeWidth="1.8"
      >
        <path d="m7 7 10 10M17 7 7 17" />
      </svg>
    </button>
  );
}
