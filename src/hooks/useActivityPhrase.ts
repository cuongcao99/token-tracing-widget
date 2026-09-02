import { useEffect, useRef, useState } from "react";
import {
  pickActivityPhrase,
  type ActivityPhraseState,
} from "../lib/activity-phrases";

const DEFAULT_MIN_INTERVAL_MS = 3_200;
const DEFAULT_MAX_INTERVAL_MS = 5_600;
const ACTIVE_MIN_INTERVAL_MS = 15_000;
const ACTIVE_MAX_INTERVAL_MS = 15_000;

export interface UseActivityPhraseOptions {
  minIntervalMs?: number;
  maxIntervalMs?: number;
  random?: () => number;
  reducedMotion?: boolean;
}

export interface ActivityPhraseResult {
  phrase: string;
  reducedMotion: boolean;
}

function prefersReducedMotion(): boolean {
  return (
    typeof window !== "undefined" &&
    typeof window.matchMedia === "function" &&
    window.matchMedia("(prefers-reduced-motion: reduce)").matches
  );
}

function clampDuration(value: number | undefined, fallback: number): number {
  return Number.isFinite(value) && value !== undefined
    ? Math.max(0, value)
    : fallback;
}

function nextDelay(
  minIntervalMs: number,
  maxIntervalMs: number,
  random: () => number,
): number {
  const min = Math.min(minIntervalMs, maxIntervalMs);
  const max = Math.max(minIntervalMs, maxIntervalMs);
  const rawValue = random();
  const normalizedValue = Number.isFinite(rawValue)
    ? Math.min(Math.max(rawValue, 0), 0.999999999)
    : 0;

  return min + (max - min) * normalizedValue;
}

export function useActivityPhrase(
  state: ActivityPhraseState,
  options: UseActivityPhraseOptions = {},
): ActivityPhraseResult {
  const random = options.random ?? Math.random;
  const defaultMinIntervalMs =
    state === "active" ? ACTIVE_MIN_INTERVAL_MS : DEFAULT_MIN_INTERVAL_MS;
  const defaultMaxIntervalMs =
    state === "active" ? ACTIVE_MAX_INTERVAL_MS : DEFAULT_MAX_INTERVAL_MS;
  const minIntervalMs = clampDuration(
    options.minIntervalMs,
    defaultMinIntervalMs,
  );
  const maxIntervalMs = clampDuration(
    options.maxIntervalMs,
    defaultMaxIntervalMs,
  );
  const [reducedMotion, setReducedMotion] = useState(
    () => options.reducedMotion ?? prefersReducedMotion(),
  );
  const [phrase, setPhrase] = useState(() =>
    pickActivityPhrase(state, undefined, random),
  );
  const previousPhraseRef = useRef(phrase);
  const previousStateRef = useRef(state);

  useEffect(() => {
    if (options.reducedMotion !== undefined) {
      setReducedMotion(options.reducedMotion);
      return;
    }

    if (typeof window === "undefined" || typeof window.matchMedia !== "function") {
      return;
    }

    const mediaQuery = window.matchMedia("(prefers-reduced-motion: reduce)");
    const updatePreference = () => setReducedMotion(mediaQuery.matches);
    updatePreference();

    if (typeof mediaQuery.addEventListener === "function") {
      mediaQuery.addEventListener("change", updatePreference);
      return () => mediaQuery.removeEventListener("change", updatePreference);
    }

    mediaQuery.addListener(updatePreference);
    return () => mediaQuery.removeListener(updatePreference);
  }, [options.reducedMotion]);

  useEffect(() => {
    if (previousStateRef.current === state) return;

    previousStateRef.current = state;
    const nextPhrase = pickActivityPhrase(state, undefined, random);
    previousPhraseRef.current = nextPhrase;
    setPhrase(nextPhrase);
  }, [random, state]);

  useEffect(() => {
    if (reducedMotion) return;

    let timeoutId: number | undefined;
    const schedule = () => {
      timeoutId = window.setTimeout(() => {
        const nextPhrase = pickActivityPhrase(
          state,
          previousPhraseRef.current,
          random,
        );
        previousPhraseRef.current = nextPhrase;
        setPhrase(nextPhrase);
        schedule();
      }, nextDelay(minIntervalMs, maxIntervalMs, random));
    };

    schedule();
    return () => {
      if (timeoutId !== undefined) window.clearTimeout(timeoutId);
    };
  }, [maxIntervalMs, minIntervalMs, random, reducedMotion, state]);

  return { phrase, reducedMotion };
}
