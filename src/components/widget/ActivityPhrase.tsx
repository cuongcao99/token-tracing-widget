import type { CSSProperties } from "react";
import {
  useActivityPhrase,
  type UseActivityPhraseOptions,
} from "../../hooks/useActivityPhrase";
import type { ActivityPhraseState } from "../../lib/activity-phrases";
import styles from "../../styles/widget/activity.module.css";

export interface ActivityPhraseProps {
  state: ActivityPhraseState;
  phrase?: string;
  options?: UseActivityPhraseOptions;
}

export default function ActivityPhrase({
  state,
  phrase: controlledPhrase,
  options,
}: ActivityPhraseProps) {
  const activityPhrase = useActivityPhrase(state, {
    ...options,
    ...(controlledPhrase ? { reducedMotion: true } : {}),
  });
  const phrase = controlledPhrase ?? activityPhrase.phrase;

  return (
    <span
      className={styles.phrase}
      data-state={state}
      data-phrase={phrase}
      data-motion={activityPhrase.reducedMotion ? "reduced" : "full"}
      aria-hidden="true"
    >
      {Array.from(phrase).map((character, index) => (
        <span
          className={styles.character}
          key={`${phrase}-${index}`}
          style={{
            animationDelay: `${index * 24}ms`,
          } as CSSProperties}
        >
          {character}
        </span>
      ))}
    </span>
  );
}
