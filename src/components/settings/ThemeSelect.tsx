import { useEffect, useId, useRef, useState } from "react";
import { themeRegistry, type ThemeId } from "../../lib/theme";

interface ThemeSelectProps {
  theme: ThemeId;
  onThemeChange: (theme: ThemeId) => void;
}

export default function ThemeSelect({
  theme,
  onThemeChange,
}: ThemeSelectProps) {
  const [isOpen, setIsOpen] = useState(false);
  const rootRef = useRef<HTMLDivElement>(null);
  const listboxId = `${useId()}-theme-options`;
  const selectedTheme =
    themeRegistry.find((option) => option.id === theme) ?? themeRegistry[0];

  useEffect(() => {
    if (!isOpen) return;

    const handlePointerDown = (event: PointerEvent) => {
      if (!rootRef.current?.contains(event.target as Node)) {
        setIsOpen(false);
      }
    };
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key !== "Escape") return;
      setIsOpen(false);
      rootRef.current?.querySelector<HTMLButtonElement>(
        ".theme-picker__button",
      )?.focus();
    };

    document.addEventListener("pointerdown", handlePointerDown);
    document.addEventListener("keydown", handleKeyDown);
    return () => {
      document.removeEventListener("pointerdown", handlePointerDown);
      document.removeEventListener("keydown", handleKeyDown);
    };
  }, [isOpen]);

  return (
    <div
      ref={rootRef}
      className={`theme-picker${isOpen ? " is-open" : ""}`}
    >
      <button
        className="theme-picker__button"
        type="button"
        aria-label={`Theme: ${selectedTheme.label}`}
        aria-haspopup="listbox"
        aria-expanded={isOpen}
        aria-controls={listboxId}
        onClick={() => setIsOpen((open) => !open)}
      >
        <span>{selectedTheme.label}</span>
        <svg
          className="theme-picker__chevron"
          viewBox="0 0 16 16"
          fill="none"
          aria-hidden="true"
        >
          <path
            d="m4 6 4 4 4-4"
            stroke="currentColor"
            strokeWidth="1.5"
            strokeLinecap="round"
            strokeLinejoin="round"
          />
        </svg>
      </button>
      {isOpen && (
        <div
          id={listboxId}
          className="theme-picker__menu"
          role="listbox"
          aria-label="Theme options"
        >
          {themeRegistry.map((option) => (
            <button
              key={option.id}
              className="theme-picker__option"
              type="button"
              role="option"
              aria-selected={option.id === selectedTheme.id}
              onClick={() => {
                onThemeChange(option.id);
                setIsOpen(false);
              }}
            >
              {option.label}
            </button>
          ))}
        </div>
      )}
    </div>
  );
}
