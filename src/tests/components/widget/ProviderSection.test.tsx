import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import type { ProviderIdentity } from "../../../lib/provider";
import ProviderSection from "../../../components/widget/ProviderSection";
import UsageMetrics from "../../../components/widget/UsageMetrics";

const fixtureIdentity: ProviderIdentity = {
  name: "Fixture Agent",
  displayName: "Fixture",
  logoSrc: "fixture-logo.svg",
  logoVariant: "warm-mark",
  fontRole: "ui",
  accent: "#123456",
};

describe("ProviderSection composition", () => {
  it("accepts an identity without a runtime provider id and preserves nested metrics", () => {
    render(
      <ProviderSection
        identity={fixtureIdentity}
        status={{ state: "active", label: "Active" }}
      >
        <UsageMetrics
          metrics={[
            { label: "First", value: "1", ariaLabel: "First: 1 tokens" },
            { label: "Second", value: "2", ariaLabel: "Second: 2 tokens" },
            { label: "Third", value: "3", ariaLabel: "Third: 3 tokens" },
          ]}
          updatedLabel="just now"
        />
      </ProviderSection>,
    );

    expect(screen.getByRole("heading", { name: "Fixture" })).toBeInTheDocument();
    expect(screen.queryByText("Active")).not.toBeInTheDocument();
    expect(screen.getByText("First")).toBeInTheDocument();
    expect(screen.getByText("1", { selector: "strong" })).toHaveAttribute(
      "aria-label",
      "First: 1 tokens",
    );
    expect(screen.getByText("Second")).toBeInTheDocument();
    expect(screen.getByText("Third")).toBeInTheDocument();
    expect(screen.getByText("just now")).toBeInTheDocument();
  });
});
