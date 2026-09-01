import type { KeyboardEvent, MouseEvent } from "react";
import { startCurrentWindowDrag } from "../../lib/window-actions";
import styles from "../../styles/shared/window-controls.module.css";

interface WindowGripProps {
  windowName: "widget" | "settings";
}

function handleMouseDown(event: MouseEvent<HTMLButtonElement>) {
  if (event.button !== 0) return;
  event.preventDefault();
  event.stopPropagation();
  void startCurrentWindowDrag().catch(() => undefined);
}

function handleKeyDown(event: KeyboardEvent<HTMLButtonElement>) {
  if (event.key !== "Enter" && event.key !== " ") return;
  event.preventDefault();
  event.stopPropagation();
  void startCurrentWindowDrag().catch(() => undefined);
}

export default function WindowGrip({ windowName }: WindowGripProps) {
  return (
    <button
      className={`${styles.grip} window-grip`}
      data-testid="window-grip"
      type="button"
      aria-label={`Move ${windowName} window`}
      onMouseDown={handleMouseDown}
      onKeyDown={handleKeyDown}
    >
      {Array.from({ length: 6 }, (_, index) => (
        <span
          className={`${styles.gripDot} window-grip__dot`}
          data-testid="window-grip-dot"
          aria-hidden="true"
          key={index}
        />
      ))}
    </button>
  );
}
