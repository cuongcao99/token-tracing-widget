import type { MouseEvent } from "react";
import {
  startCurrentWindowResize,
  type WindowResizeDirection,
} from "../../lib/window-actions";
import styles from "../../styles/shared/window-controls.module.css";

interface WindowResizeHandlesProps {
  windowName: "widget" | "settings";
}

const resizeHandles: Array<{
  direction: WindowResizeDirection;
  edge: string;
  label: string;
}> = [
  { direction: "North", edge: "n", label: "top edge" },
  { direction: "NorthEast", edge: "ne", label: "top-right corner" },
  { direction: "East", edge: "e", label: "right edge" },
  { direction: "SouthEast", edge: "se", label: "bottom-right corner" },
  { direction: "South", edge: "s", label: "bottom edge" },
  { direction: "SouthWest", edge: "sw", label: "bottom-left corner" },
  { direction: "West", edge: "w", label: "left edge" },
  { direction: "NorthWest", edge: "nw", label: "top-left corner" },
];

function handleResizeStart(
  event: MouseEvent<HTMLButtonElement>,
  direction: WindowResizeDirection,
) {
  if (event.button !== 0) return;
  event.preventDefault();
  event.stopPropagation();
  void startCurrentWindowResize(direction).catch(() => undefined);
}

export default function WindowResizeHandles({ windowName }: WindowResizeHandlesProps) {
  return (
    <div
      className={`${styles.resizeHandles} window-resize-handles`}
      aria-label={`${windowName} resize handles`}
    >
      {resizeHandles.map(({ direction, edge, label }) => (
        <button
          className={`${styles.resizeHandle} ${styles[`resizeHandle${edge.toUpperCase()}` as keyof typeof styles]} window-resize-handle window-resize-handle--${edge}`}
          key={direction}
          type="button"
          aria-label={`Resize ${windowName} from ${label}`}
          onMouseDown={(event) => handleResizeStart(event, direction)}
        />
      ))}
    </div>
  );
}
