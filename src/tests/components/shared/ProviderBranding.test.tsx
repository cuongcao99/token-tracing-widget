import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import ProviderBrand from "../../../components/shared/ProviderBrand";
import ProviderDot from "../../../components/shared/ProviderDot";
import ProviderName from "../../../components/shared/ProviderName";
import { providerMeta } from "../../../lib/provider";

afterEach(() => cleanup());

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
    expect(container.firstElementChild).toHaveAttribute(
      "data-logo-variant",
      "monochrome-mark",
    );
  });

  it("renders registry branding metadata without a provider-specific branch", () => {
    render(<ProviderBrand identity={providerMeta.claude} />);

    const brand = document.querySelector<HTMLElement>(".provider-brand")!;
    expect(brand).toHaveAttribute("data-logo-variant", "warm-mark");
    expect(brand).toHaveAttribute("data-font-role", "display");
    expect(brand).toHaveStyle({ "--provider-accent": "#cc785c" });
    expect(brand.querySelector("img")).toHaveAttribute(
      "src",
      expect.stringMatching(/^data:image\/svg\+xml/),
    );
    expect(screen.getByText("Claude")).toBeInTheDocument();
  });

  it("renders Claude as the concise display name", () => {
    const { container } = render(<ProviderName provider="claude" />);

    expect(screen.getByText("Claude")).toHaveAttribute(
      "data-logo-variant",
      "warm-mark",
    );
    expect(screen.getByText("Claude")).toHaveAttribute("data-font-role", "display");
    expect(container).not.toHaveTextContent("Claude Code");
  });
});
