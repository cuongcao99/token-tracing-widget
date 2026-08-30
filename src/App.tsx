import { useEffect, useState } from "react";
import { type UnlistenFn } from "@tauri-apps/api/event";
import {
  formatRelativeUpdate,
  getUsageSummary,
  listenForUsageSummary,
  type UsageSummary,
} from "./lib/usage-summary";

const loadingSummary: UsageSummary = {
  state: "loading",
  todayTokens: 0,
  sourceHealth: [],
};

const unavailableSummary: UsageSummary = {
  state: "unavailable",
  todayTokens: 0,
  sourceHealth: [],
};

function stateLabel(state: UsageSummary["state"]): string {
  return state.charAt(0).toUpperCase() + state.slice(1);
}

function formatTokens(tokens: number): string {
  return tokens.toLocaleString("en-US");
}

export default function App() {
  const [summary, setSummary] = useState<UsageSummary>(loadingSummary);

  useEffect(() => {
    let mounted = true;
    let unlisten: UnlistenFn | undefined;

    const connect = async () => {
      try {
        const stop = await listenForUsageSummary((nextSummary) => {
          if (mounted) {
            setSummary(nextSummary);
          }
        });
        if (!mounted) {
          void stop();
          return;
        }
        unlisten = stop;
      } catch {
        if (mounted) {
          setSummary(unavailableSummary);
        }
      }

      try {
        const initialSummary = await getUsageSummary();
        if (mounted) {
          setSummary(initialSummary);
        }
      } catch {
        if (mounted) {
          setSummary(unavailableSummary);
        }
      }
    };

    void connect();
    return () => {
      mounted = false;
      if (unlisten) {
        void unlisten();
      }
    };
  }, []);

  const currentSession =
    summary.currentSessionTokens === undefined
      ? "Unavailable"
      : `${formatTokens(summary.currentSessionTokens)} tokens`;

  return (
    <main className="widget" aria-label="Token usage summary">
      <header
        className="widget__header"
        data-tauri-drag-region=""
      >
        <h1>{summary.provider ?? "Token Tracing"}</h1>
        <span className={`status status--${summary.state}`}>
          {stateLabel(summary.state)}
        </span>
      </header>

      <section className="widget__metrics" aria-label="Usage totals">
        <div>
          <span className="metric__label">Current session</span>
          <strong>{currentSession}</strong>
        </div>
        <div>
          <span className="metric__label">Today</span>
          <strong>Today: {formatTokens(summary.todayTokens)} tokens</strong>
        </div>
      </section>

      <p className="widget__note">
        {formatRelativeUpdate(summary.lastUpdatedAt)}
      </p>
    </main>
  );
}
