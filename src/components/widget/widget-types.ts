import type { WidgetProviderViewModel } from "../../lib/widget-view-model";

export type { WidgetProviderViewModel } from "../../lib/widget-view-model";
export { stateLabel } from "../../lib/widget-view-model";

export interface WidgetProviderRowProps {
  usage: WidgetProviderViewModel;
  onSessionToggle?: () => void;
}

export interface UsageMetric {
  label: string;
  value: string;
  ariaLabel: string;
}

export function formatTokens(tokens: number | undefined): string {
  return tokens === undefined ? "Unavailable" : tokens.toLocaleString("en-US");
}
