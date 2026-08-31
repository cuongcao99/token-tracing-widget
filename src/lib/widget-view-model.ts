import { formatRelativeUpdate, type ProviderUsageSummary, type UsageState, type UsageSummary } from "./usage-summary";
import { providerMeta, providerOrder, type ProviderId, type ProviderIdentity } from "./provider";
import type { ThemeId } from "./theme";
import type { WidgetSettingsSnapshot } from "./widget-settings";

export interface WidgetViewModelInput {
  summary: UsageSummary;
  settings: WidgetSettingsSnapshot;
  previewSourceEnabled: Readonly<Record<ProviderId, boolean>> | null;
}

export interface WidgetProviderViewModel {
  provider: ProviderId;
  identity: ProviderIdentity;
  status: { state: UsageState; label: string };
  metrics: {
    sessionTokens?: number;
    todayTokens: number;
    updatedLabel: string;
  };
}

export interface WidgetViewModel {
  theme: ThemeId;
  colorMode: "dark" | "light";
  providers: WidgetProviderViewModel[];
  totalTokens: number;
  visibleProviderCount: number;
}

export function stateLabel(state: UsageState): string {
  return state.charAt(0).toUpperCase() + state.slice(1);
}

function visibleProviderSet(settings: WidgetSettingsSnapshot): Set<ProviderId> {
  return new Set(
    settings.visibleProviders
      .filter(({ visible }) => visible)
      .map(({ provider }) => provider),
  );
}

function providerSummaryIndex(
  summaries: readonly ProviderUsageSummary[],
): Map<ProviderId, ProviderUsageSummary> {
  return new Map(summaries.map((usage) => [usage.provider, usage]));
}

function viewForProvider(
  provider: ProviderId,
  usage: ProviderUsageSummary,
  previewSourceEnabled: Readonly<Record<ProviderId, boolean>> | null,
): WidgetProviderViewModel {
  const previewDisabled = previewSourceEnabled?.[provider] === false;
  const state = previewDisabled ? "unavailable" : usage.state;
  return {
    provider,
    identity: providerMeta[provider],
    status: { state, label: stateLabel(state) },
    metrics: {
      ...(usage.currentSessionTokens === undefined
        ? {}
        : { sessionTokens: usage.currentSessionTokens }),
      todayTokens: usage.todayTokens,
      updatedLabel: formatRelativeUpdate(usage.lastUpdatedAt),
    },
  };
}

export function createWidgetViewModel({
  summary,
  settings,
  previewSourceEnabled,
}: WidgetViewModelInput): WidgetViewModel {
  const visibleProviders = visibleProviderSet(settings);
  const usageByProvider = providerSummaryIndex(summary.providers);
  const providers: WidgetProviderViewModel[] = [];

  for (const provider of providerOrder) {
    const usage = usageByProvider.get(provider);
    if (usage && visibleProviders.has(provider)) {
      providers.push(viewForProvider(provider, usage, previewSourceEnabled));
    }
  }

  const totalTokens =
    previewSourceEnabled === null
      ? summary.todayTokens
      : summary.providers.reduce(
          (total, usage) =>
            previewSourceEnabled[usage.provider] === false
              ? total
              : total + usage.todayTokens,
          0,
        );

  return {
    theme: settings.theme,
    colorMode: settings.darkMode ? "dark" : "light",
    providers,
    totalTokens,
    visibleProviderCount: providers.length,
  };
}
