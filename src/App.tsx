import { useEffect, useState } from "react";
import { getUsageSummary, type UsageSummary } from "./lib/usage-summary";

const loadingSummary: UsageSummary = {
  state: "loading",
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

    void getUsageSummary()
      .then((nextSummary) => {
        if (mounted) {
          setSummary(nextSummary);
        }
      })
      .catch(() => {
        if (mounted) {
          setSummary({
            state: "unavailable",
            todayTokens: 0,
            sourceHealth: [],
          });
        }
      });

    return () => {
      mounted = false;
    };
  }, []);

  const currentSession =
    summary.currentSessionTokens === undefined
      ? "Unavailable"
      : `${formatTokens(summary.currentSessionTokens)} tokens`;

  return (
    <main className="widget" aria-label="Token usage summary">
      <header className="widget__header">
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

      <p className="widget__note">Bootstrap shell; collection is not enabled yet.</p>
    </main>
  );
}
