import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import SessionUsageList from "../../../components/widget/SessionUsageList";

describe("SessionUsageList", () => {
  it("keeps the active marker and uses a neutral disclosure label", () => {
    render(
      <SessionUsageList
        sessions={[
          { id: "active-id", label: "Active run", state: "active", todayTokens: 12 },
          { id: "idle-id", label: "Idle run", state: "idle", todayTokens: 8 },
        ]}
      />,
    );

    expect(screen.getByText("Active run")).toBeInTheDocument();
    expect(screen.getByText("Current")).toBeInTheDocument();
    expect(
      screen.getByRole("group", { name: "Current session, Active run: 12 tokens" }),
    ).toBeInTheDocument();
    expect(screen.queryByText("Idle · 1")).not.toBeInTheDocument();
    const disclosure = screen.getByText("More sessions").closest("details");
    expect(disclosure).not.toBeNull();
    expect(disclosure).not.toHaveAttribute("open");

    fireEvent.click(screen.getByText("More sessions"));

    expect(disclosure).toHaveAttribute("open");
    expect(screen.getByText("Idle run")).toBeInTheDocument();
    expect(screen.getByRole("group", { name: "Idle run: 8 tokens" })).toBeInTheDocument();
  });

  it("shortens fallback IDs while exposing and copying the full ID", () => {
    const fullId = "70f99f1570e0b461165bed62a60aa9a703d402fe8b54edb98fd7c69b496b03f2";
    const writeText = vi.fn().mockResolvedValue(undefined);
    const previousClipboard = navigator.clipboard;
    Object.defineProperty(navigator, "clipboard", {
      configurable: true,
      value: { writeText },
    });

    try {
      render(
        <SessionUsageList
          sessions={[{ id: fullId, label: fullId, state: "active", todayTokens: 12 }]}
        />,
      );

      expect(screen.getByText("70f99f15…b03f2")).toBeInTheDocument();
      expect(screen.queryByText(fullId)).not.toBeInTheDocument();
      const copyButton = screen.getByRole("button", {
        name: `Copy session ID ${fullId}`,
      });
      expect(copyButton).toHaveAttribute("title", fullId);

      fireEvent.click(copyButton);

      expect(writeText).toHaveBeenCalledWith(fullId);
    } finally {
      Object.defineProperty(navigator, "clipboard", {
        configurable: true,
        value: previousClipboard,
      });
    }
  });

  it("does not add a token-scaled background to session rows", () => {
    render(
      <SessionUsageList
        sessions={[
          { id: "largest", label: "Largest", state: "active", todayTokens: 100 },
          { id: "small", label: "Small", state: "active", todayTokens: 25 },
        ]}
      />,
    );

    expect(
      screen.getByRole("group", { name: "Current session, Largest: 100 tokens" }),
    ).not.toHaveAttribute("style");
    expect(
      screen.getByRole("group", { name: "Current session, Small: 25 tokens" }),
    ).not.toHaveAttribute("style");
  });
});
