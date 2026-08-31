import { getCurrentWindow } from "@tauri-apps/api/window";

export type WindowResizeDirection =
  | "East"
  | "North"
  | "NorthEast"
  | "NorthWest"
  | "South"
  | "SouthEast"
  | "SouthWest"
  | "West";

export function startCurrentWindowDrag(): Promise<void> {
  return getCurrentWindow().startDragging();
}

export function startCurrentWindowResize(
  direction: WindowResizeDirection,
): Promise<void> {
  return getCurrentWindow().startResizeDragging(direction);
}

export function closeCurrentWindow(): Promise<void> {
  return getCurrentWindow().close();
}
