export type ActivityPhraseState =
  | "loading"
  | "active"
  | "stopping"
  | "idle"
  | "stale"
  | "unavailable";

export const ACTIVITY_PHRASES = {
  loading: ["Waking the tokens", "Finding the context", "Booting the vibes"],
  active: [
    "Token maxing",
    "Over-engineering",
    "Flibbitigibiting",
    "Summoning context",
    "Thinking in markdown",
  ],
  stopping: ["Cooling down", "Counting the damage", "Packaging thoughts"],
  idle: ["Hibernating", "Waiting politely", "Conserving context"],
  stale: ["Rehydrating", "Checking the receipts", "Recovering context"],
  unavailable: [
    "Looking for the provider",
    "Searching the vibes",
    "Agent not found",
  ],
} as const satisfies Record<ActivityPhraseState, readonly string[]>;

export function pickActivityPhrase(
  state: ActivityPhraseState,
  previousPhrase?: string,
  random: () => number = Math.random,
): string {
  const phrases = ACTIVITY_PHRASES[state];
  const candidates =
    phrases.length > 1 && previousPhrase
      ? phrases.filter((phrase) => phrase !== previousPhrase)
      : phrases;
  const rawValue = random();
  const normalizedValue = Number.isFinite(rawValue)
    ? Math.min(Math.max(rawValue, 0), 0.999999999)
    : 0;
  const index = Math.floor(normalizedValue * candidates.length);

  return candidates[index] ?? phrases[0] ?? "Thinking...";
}
