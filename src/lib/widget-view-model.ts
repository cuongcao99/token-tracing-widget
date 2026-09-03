import {
  formatRelativeUpdate,
  type RateLimitSummary,
  type ProviderUsageSummary,
  type SessionUsageState,
  type UsageSummary,
} from "./usage-summary";
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
  sessions: WidgetSessionViewModel[];
  sessionCount: number;
  rateLimits: RateLimitSummary[];
  metrics: {
    sessionTokens?: number;
    todayTokens: number;
    updatedLabel: string;
  };
}

export interface WidgetSessionViewModel {
  id: string;
  label: string;
  state: SessionUsageState;
  todayTokens: number;
}

export interface WidgetViewModel {
  theme: ThemeId;
  colorMode: "dark" | "light";
  providers: WidgetProviderViewModel[];
  totalTokens: number;
  visibleProviderCount: number;
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

function viewForSession({
  id,
  name,
  state,
  todayTokens,
}: ProviderUsageSummary["sessions"][number]): WidgetSessionViewModel {
  return {
    id,
    label: name ?? id,
    state,
    todayTokens,
  };
}

function viewForProvider(
  provider: ProviderId,
  usage: ProviderUsageSummary,
): WidgetProviderViewModel {
  return {
    provider,
    identity: providerMeta[provider],
    sessions: usage.sessions.map(viewForSession),
    sessionCount: usage.sessions.length,
    rateLimits: usage.rateLimits ?? [],
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
      providers.push(viewForProvider(provider, usage));
    }
  }

  const totalTokens = providers.reduce(
    (total, provider) =>
      previewSourceEnabled?.[provider.provider] === false
        ? total
        : total + provider.metrics.todayTokens,
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
