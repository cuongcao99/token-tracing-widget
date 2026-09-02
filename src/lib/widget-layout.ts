export const WIDGET_DEFAULT_WIDTH = 440;
export const WIDGET_MIN_WIDTH = 360;
export const WIDGET_MAX_WIDTH = 720;
export const WIDGET_MIN_HEIGHT = 192;
export const WIDGET_MAX_HEIGHT = 520;
const WIDGET_CONTENT_ANCHOR_GAP = 17;

const WIDGET_HEIGHTS_BY_VISIBLE_PROVIDER_COUNT = [192, 244, 316] as const;

export function widgetHeightForVisibleProviders(visibleProviderCount: number): number {
  if (!Number.isFinite(visibleProviderCount)) return WIDGET_MIN_HEIGHT;

  const index = Math.max(
    0,
    Math.min(
      WIDGET_HEIGHTS_BY_VISIBLE_PROVIDER_COUNT.length - 1,
      Math.floor(visibleProviderCount),
    ),
  );
  return WIDGET_HEIGHTS_BY_VISIBLE_PROVIDER_COUNT[index];
}

export function widgetHeightForContent(
  visibleProviderCount: number,
  measuredContentHeight?: number,
): number {
  const minimum = widgetHeightForVisibleProviders(visibleProviderCount);
  if (measuredContentHeight === undefined || !Number.isFinite(measuredContentHeight)) {
    return minimum;
  }
  return Math.max(
    minimum,
    Math.min(
      WIDGET_MAX_HEIGHT,
      Math.ceil(measuredContentHeight) + WIDGET_CONTENT_ANCHOR_GAP,
    ),
  );
}
