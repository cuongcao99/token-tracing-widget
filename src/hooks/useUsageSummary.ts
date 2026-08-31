import { useEffect, useState } from "react";
import type { UnlistenFn } from "@tauri-apps/api/event";
import {
  getUsageSummary,
  listenForUsageSummary,
  type UsageState,
  type UsageSummary,
} from "../lib/usage-summary";
import { providerOrder } from "../lib/provider";

function fallbackSummary(state: UsageState): UsageSummary {
  return {
    state,
    todayTokens: 0,
    sourceHealth: [],
    providers: providerOrder.map((provider) => ({
      provider,
      state,
      todayTokens: 0,
    })),
  };
}

export const loadingSummary = fallbackSummary("loading");
export const unavailableSummary = fallbackSummary("unavailable");

export function useUsageSummary(): { summary: UsageSummary } {
  const [summary, setSummary] = useState<UsageSummary>(loadingSummary);

  useEffect(() => {
    let mounted = true;
    let unlisten: UnlistenFn | undefined;

    const connect = async () => {
      try {
        const stop = await listenForUsageSummary((nextSummary) => {
          if (mounted) setSummary(nextSummary);
        });
        if (!mounted) {
          void stop();
        } else {
          unlisten = stop;
        }
      } catch {
        // The initial command remains the source of truth when event setup is unavailable.
      }

      try {
        const initialSummary = await getUsageSummary();
        if (mounted) setSummary(initialSummary);
      } catch {
        if (mounted) setSummary(unavailableSummary);
      }
    };

    void connect();
    return () => {
      mounted = false;
      if (unlisten) void unlisten();
    };
  }, []);

  return { summary };
}
