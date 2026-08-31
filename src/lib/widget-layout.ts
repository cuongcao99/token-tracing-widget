export const WIDGET_DEFAULT_WIDTH = 440;
export const WIDGET_MIN_WIDTH = 360;
export const WIDGET_MAX_WIDTH = 720;
export const WIDGET_MIN_HEIGHT = 176;
export const WIDGET_MAX_HEIGHT = 520;

const WIDGET_HEIGHTS_BY_VISIBLE_PROVIDER_COUNT = [176, 228, 300] as const;

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
