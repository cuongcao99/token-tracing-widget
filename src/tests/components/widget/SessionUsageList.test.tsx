import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import SessionUsageList from "../../../components/widget/SessionUsageList";

describe("SessionUsageList", () => {
  it("shows active sessions and keeps idle sessions behind a collapsed disclosure", () => {
    render(
      <SessionUsageList
        sessions={[
          { id: "active-id", label: "Active run", state: "active", todayTokens: 12 },
          { id: "idle-id", label: "Idle run", state: "idle", todayTokens: 8 },
        ]}
      />,
    );

    expect(screen.getByText("Active run")).toBeInTheDocument();
    expect(screen.getByText("12")).toBeInTheDocument();
    const disclosure = screen.getByText("Idle · 1").closest("details");
    expect(disclosure).not.toBeNull();
    expect(disclosure).not.toHaveAttribute("open");

    fireEvent.click(screen.getByText("Idle · 1"));

    expect(disclosure).toHaveAttribute("open");
    expect(screen.getByText("Idle run")).toBeInTheDocument();
    expect(screen.getByText("8")).toBeInTheDocument();
  });
});
