import {
  getCurrentWindow,
  LogicalSize,
} from "@tauri-apps/api/window";
import {
  WIDGET_MAX_HEIGHT,
  WIDGET_MAX_WIDTH,
  WIDGET_MIN_WIDTH,
  widgetHeightForVisibleProviders,
} from "../widget-layout";

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

function clamp(value: number, minimum: number, maximum: number): number {
  return Math.max(minimum, Math.min(maximum, value));
}

let latestRequest = 0;
let resizeQueue: Promise<void> = Promise.resolve();

export function syncWidgetWindowHeight(
  visibleProviderCount: number,
): Promise<void> {
  const requestId = ++latestRequest;
  const request = resizeQueue.catch(() => undefined).then(async () => {
    const window = getCurrentWindow();
    const [physicalSize, scaleFactor] = await Promise.all([
      window.innerSize(),
      window.scaleFactor(),
    ]);

    if (requestId !== latestRequest) return;

    const factor = Number.isFinite(scaleFactor) && scaleFactor > 0 ? scaleFactor : 1;
    const targetHeight = widgetHeightForVisibleProviders(visibleProviderCount);
    const logicalWidth = clamp(
      Math.round(physicalSize.width / factor),
      WIDGET_MIN_WIDTH,
      WIDGET_MAX_WIDTH,
    );

    await window.setSizeConstraints({
      minWidth: WIDGET_MIN_WIDTH,
      minHeight: targetHeight,
      maxWidth: WIDGET_MAX_WIDTH,
      maxHeight: WIDGET_MAX_HEIGHT,
    });

    if (requestId !== latestRequest) return;

    await window.setSize(new LogicalSize(logicalWidth, targetHeight));
  });

  resizeQueue = request.catch(() => undefined);
  return request;
}
