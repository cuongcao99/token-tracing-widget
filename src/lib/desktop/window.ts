import {
  getCurrentWindow,
  LogicalSize,
} from "@tauri-apps/api/window";
import {
  WIDGET_MAX_HEIGHT,
  WIDGET_MAX_WIDTH,
  WIDGET_MIN_WIDTH,
  widgetHeightForContent,
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
const WIDGET_RESIZE_DURATION_MS = 150;

function prefersReducedMotion(): boolean {
  return (
    typeof globalThis.matchMedia === "function" &&
    globalThis.matchMedia("(prefers-reduced-motion: reduce)").matches
  );
}

function nextAnimationFrame(): Promise<number> {
  if (typeof globalThis.requestAnimationFrame === "function") {
    return new Promise((resolve) => {
      globalThis.requestAnimationFrame((timestamp) => resolve(timestamp));
    });
  }

  return new Promise((resolve) => {
    globalThis.setTimeout(() => resolve(Date.now()), 16);
  });
}

async function setWidgetSize(
  window: ReturnType<typeof getCurrentWindow>,
  logicalWidth: number,
  currentHeight: number,
  targetHeight: number,
  requestId: number,
  animate: boolean,
): Promise<void> {
  if (!animate || prefersReducedMotion() || currentHeight === targetHeight) {
    await window.setSize(new LogicalSize(logicalWidth, targetHeight));
    return;
  }

  const startTimestamp = await nextAnimationFrame();
  let timestamp = startTimestamp;
  let lastSizeRequest = Promise.resolve();

  while (true) {
    if (requestId !== latestRequest) {
      await lastSizeRequest;
      return;
    }

    const progress = Math.min(
      1,
      Math.max(0, (timestamp - startTimestamp) / WIDGET_RESIZE_DURATION_MS),
    );
    const height = Math.round(
      currentHeight + (targetHeight - currentHeight) * progress,
    );

    if (height !== currentHeight || progress >= 1) {
      lastSizeRequest = window
        .setSize(new LogicalSize(logicalWidth, height))
        .catch(() => undefined);
    }

    if (progress >= 1) {
      await lastSizeRequest;
      return;
    }
    timestamp = await nextAnimationFrame();
  }
}

export function syncWidgetWindowHeight(
  visibleProviderCount: number,
  measuredContentHeight?: number,
  animate = false,
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
    const targetHeight = widgetHeightForContent(
      visibleProviderCount,
      measuredContentHeight,
    );
    const minimumHeight = widgetHeightForVisibleProviders(visibleProviderCount);
    const logicalWidth = clamp(
      Math.round(physicalSize.width / factor),
      WIDGET_MIN_WIDTH,
      WIDGET_MAX_WIDTH,
    );

    await window.setSizeConstraints({
      minWidth: WIDGET_MIN_WIDTH,
      minHeight: minimumHeight,
      maxWidth: WIDGET_MAX_WIDTH,
      maxHeight: WIDGET_MAX_HEIGHT,
    });

    if (requestId !== latestRequest) return;

    const currentHeight = clamp(
      Math.round(physicalSize.height / factor),
      minimumHeight,
      WIDGET_MAX_HEIGHT,
    );
    await setWidgetSize(
      window,
      logicalWidth,
      currentHeight,
      targetHeight,
      requestId,
      animate,
    );
  });

  resizeQueue = request.catch(() => undefined);
  return request;
}
