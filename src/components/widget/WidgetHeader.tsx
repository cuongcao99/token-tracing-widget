import styles from "../../styles/widget/surface.module.css";
import type { UsageState } from "../../lib/contracts/usage-summary";
import ActivityPhrase from "./ActivityPhrase";
import type { ActivityPhraseState } from "../../lib/activity-phrases";

interface WidgetHeaderProps {
  activityState?: UsageState;
}

function toActivityPhraseState(state: UsageState): ActivityPhraseState {
  return state;
}

export default function WidgetHeader({
  activityState = "loading",
}: WidgetHeaderProps) {
  return (
    <header className={styles.header}>
      <div className={styles.title}>
        <h1 className={styles.titleText}>Token Tracing</h1>
      </div>
      <ActivityPhrase state={toActivityPhraseState(activityState)} />
    </header>
  );
}
