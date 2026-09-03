import { syncWidgetWindowHeight as syncDesktopWidgetWindowHeight } from "./desktop/window";

export function syncWidgetWindowHeight(
  visibleProviderCount: number,
  measuredContentHeight?: number,
  animate = false,
): Promise<void> {
  return syncDesktopWidgetWindowHeight(
    visibleProviderCount,
    measuredContentHeight,
    animate,
  );
}
