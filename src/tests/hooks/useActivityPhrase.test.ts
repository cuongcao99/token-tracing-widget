import { act, renderHook } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { useActivityPhrase } from "../../hooks/useActivityPhrase";

afterEach(() => {
  vi.useRealTimers();
});

describe("useActivityPhrase", () => {
  it("keeps non-active phrases for a slower default cadence", () => {
    vi.useFakeTimers();
    const random = vi.fn(() => 0);

    const { result } = renderHook(() => useActivityPhrase("idle", { random }));
    const firstPhrase = result.current.phrase;

    act(() => {
      vi.advanceTimersByTime(7_999);
    });
    expect(result.current.phrase).toBe(firstPhrase);

    act(() => {
      vi.advanceTimersByTime(1);
    });
    expect(result.current.phrase).not.toBe(firstPhrase);
  });

  it("keeps an active phrase for about fifteen seconds by default", () => {
    vi.useFakeTimers();
    const random = vi.fn(() => 0);

    const { result } = renderHook(() => useActivityPhrase("active", { random }));
    const firstPhrase = result.current.phrase;

    act(() => {
      vi.advanceTimersByTime(14_999);
    });
    expect(result.current.phrase).toBe(firstPhrase);

    act(() => {
      vi.advanceTimersByTime(1);
    });
    expect(result.current.phrase).not.toBe(firstPhrase);
  });

  it("rotates to another phrase after the configured delay", () => {
    vi.useFakeTimers();
    const randomValues = [0, 0.9];
    const random = vi.fn(() => randomValues.shift() ?? 0.9);

    const { result } = renderHook(() =>
      useActivityPhrase("active", {
        minIntervalMs: 100,
        maxIntervalMs: 100,
        random,
      }),
    );
    const firstPhrase = result.current.phrase;

    act(() => {
      vi.advanceTimersByTime(100);
    });

    expect(result.current.phrase).not.toBe(firstPhrase);
  });

  it("does not auto-rotate when reduced motion is requested", () => {
    vi.useFakeTimers();
    const random = vi.fn(() => 0);

    const { result } = renderHook(() =>
      useActivityPhrase("active", {
        minIntervalMs: 100,
        maxIntervalMs: 100,
        random,
        reducedMotion: true,
      }),
    );
    const firstPhrase = result.current.phrase;

    act(() => {
      vi.advanceTimersByTime(500);
    });

    expect(result.current.phrase).toBe(firstPhrase);
  });
});
