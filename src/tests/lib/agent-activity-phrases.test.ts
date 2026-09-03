import { describe, expect, it } from "vitest";
import {
  ACTIVITY_PHRASES,
  pickActivityPhrase,
  type ActivityPhraseState,
} from "../../lib/activity-phrases";

const states = [
  "active",
  "stopping",
  "idle",
  "stale",
  "unavailable",
] as const satisfies readonly ActivityPhraseState[];

describe("activity phrase catalog", () => {
  it.each(states)("provides at least one phrase for %s", (state) => {
    expect(ACTIVITY_PHRASES[state].length).toBeGreaterThan(0);
    expect(ACTIVITY_PHRASES[state].every((phrase) => phrase.trim().length > 0)).toBe(true);
  });

  it("chooses a different phrase when the previous phrase has an alternative", () => {
    const previousPhrase = "Token maxing";

    expect(ACTIVITY_PHRASES.active).toContain(previousPhrase);
    expect(ACTIVITY_PHRASES.active.length).toBeGreaterThan(1);

    const nextPhrase = pickActivityPhrase("active", previousPhrase, () => 0);

    expect(ACTIVITY_PHRASES.active).toContain(nextPhrase);
    expect(nextPhrase).not.toBe(previousPhrase);
  });
});
