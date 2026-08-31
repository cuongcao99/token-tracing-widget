import {
  closeCurrentWindow as closeDesktopWindow,
  startCurrentWindowDrag as startDesktopWindowDrag,
  startCurrentWindowResize as startDesktopWindowResize,
  type WindowResizeDirection,
} from "./desktop/window";

export type { WindowResizeDirection } from "./desktop/window";

export function startCurrentWindowDrag(): Promise<void> {
  return startDesktopWindowDrag();
}

export function startCurrentWindowResize(
  direction: WindowResizeDirection,
): Promise<void> {
  return startDesktopWindowResize(direction);
}

export function closeCurrentWindow(): Promise<void> {
  return closeDesktopWindow();
}
