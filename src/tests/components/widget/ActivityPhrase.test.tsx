import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import ActivityPhrase from "../../../components/widget/ActivityPhrase";

const states = [
  "active",
  "stopping",
  "idle",
  "stale",
  "unavailable",
] as const;

afterEach(() => cleanup());

function getPhraseElement(state: (typeof states)[number]): HTMLElement {
  const phrase = document.querySelector<HTMLElement>(
    `[data-state="${state}"]`,
  );
  if (!phrase) throw new Error(`Missing activity phrase for ${state}`);
  return phrase;
}

describe("ActivityPhrase", () => {
  it.each(states)("renders the phrase and exposes the %s state", (state) => {
    render(<ActivityPhrase state={state} phrase="Token maxing" />);

    const phrase = getPhraseElement(state);

    expect(phrase).toHaveAttribute("data-state", state);
    expect(phrase).toHaveAttribute("data-phrase", "Token maxing");
    expect(phrase).toHaveTextContent("Token maxing");
    expect(screen.queryByRole("status")).not.toBeInTheDocument();
  });

  it("keeps the phrase as ordinary visible text for reduced-motion presentation", () => {
    render(<ActivityPhrase state="active" phrase="Token maxing" />);

    expect(getPhraseElement("active")).toBeVisible();
  });
});
