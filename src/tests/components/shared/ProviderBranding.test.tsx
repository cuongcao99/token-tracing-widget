import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import ProviderDot from "../../../components/shared/ProviderDot";
import ProviderName from "../../../components/shared/ProviderName";

describe("provider branding", () => {
  it("renders local Claude and OpenAI provider marks", () => {
    const { container, rerender } = render(<ProviderDot provider="claude" />);

    expect(container.querySelector("img")).toHaveAttribute(
      "src",
      expect.stringMatching(/^data:image\/svg\+xml/),
    );

    rerender(<ProviderDot provider="codex" />);
    expect(container.querySelector("img")).toHaveAttribute(
      "src",
      expect.stringMatching(/^data:image\/svg\+xml/),
    );
  });

  it("renders Claude as the concise display name", () => {
    const { container } = render(<ProviderName provider="claude" />);

    expect(screen.getByText("Claude")).toHaveClass(
      "provider-name",
      "provider-name--claude",
      "provider-name--font-display",
    );
    expect(container).not.toHaveTextContent("Claude Code");
  });
});
