import { getCurrentWindow, LogicalSize } from "@tauri-apps/api/window";
import {
  WIDGET_MAX_WIDTH,
  WIDGET_MIN_WIDTH,
  widgetHeightForVisibleProviders,
} from "./widget-layout";

let latestRequest = 0;
let resizeQueue: Promise<void> = Promise.resolve();

function clamp(value: number, minimum: number, maximum: number): number {
  return Math.max(minimum, Math.min(maximum, value));
}

export function syncWidgetWindowHeight(visibleProviderCount: number): Promise<void> {
  const requestId = ++latestRequest;
  const request = resizeQueue.catch(() => undefined).then(async () => {
    const window = getCurrentWindow();
    const [physicalSize, scaleFactor] = await Promise.all([
      window.innerSize(),
      window.scaleFactor(),
    ]);

    if (requestId !== latestRequest) return;

    const factor = Number.isFinite(scaleFactor) && scaleFactor > 0 ? scaleFactor : 1;
    const logicalWidth = clamp(
      Math.round(physicalSize.width / factor),
      WIDGET_MIN_WIDTH,
      WIDGET_MAX_WIDTH,
    );
    await window.setSize(
      new LogicalSize(logicalWidth, widgetHeightForVisibleProviders(visibleProviderCount)),
    );
  });

  resizeQueue = request.catch(() => undefined);
  return request;
}
