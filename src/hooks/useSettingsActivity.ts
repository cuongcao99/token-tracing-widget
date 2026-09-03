import { useUsageSummary } from "./useUsageSummary";
import { providerOrder, type ProviderId } from "../lib/provider";
import {
  formatRelativeUpdate,
  type UsageSummary,
} from "../lib/usage-summary";
import type { ProviderStatusView } from "../components/settings/settings-types";

export interface SettingsActivity {
  summary: UsageSummary;
  providerStatuses: ProviderStatusView[];
}

export function useSettingsActivity(): SettingsActivity {
  const { summary } = useUsageSummary();
  const providerStatuses = providerOrder.map((provider: ProviderId) => {
    const usage = summary.providers.find(
      (entry) => entry.provider === provider,
    );
    return {
      provider,
      state: usage?.state ?? "unavailable",
      updated: usage?.lastUpdatedAt
        ? formatRelativeUpdate(usage.lastUpdatedAt)
        : "No updates yet",
    };
  });

  return { summary, providerStatuses };
}

export default useSettingsActivity;
