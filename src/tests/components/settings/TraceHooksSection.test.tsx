import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import TraceHooksSection from "../../../components/settings/TraceHooksSection";
import styles from "../../../styles/settings/trace-hooks.module.css";

const statuses = [
  { provider: "claude" as const, state: "not_installed" as const, requiresTrust: false },
  { provider: "codex" as const, state: "configured" as const, requiresTrust: true },
];

afterEach(cleanup);

function renderSection(
  overrides: Partial<React.ComponentProps<typeof TraceHooksSection>> = {},
) {
  return render(
    <TraceHooksSection
      statuses={statuses}
      loading={false}
      error={null}
      updatingProvider={null}
      onToggle={vi.fn()}
      {...overrides}
    />,
  );
}

describe("TraceHooksSection", () => {
  it("renders provider-owned hook setup without claiming Codex is active", () => {
    const onToggle = vi.fn();
    renderSection({ onToggle });

    const section = screen.getByRole("heading", { name: "Agent tracing" }).closest("section");
    expect(section).toHaveClass(styles.section);
    expect(screen.getByText("Show lightweight live activity from your coding agents.")).toBeInTheDocument();

    expect(screen.getByRole("switch", { name: "Configure Claude live tracing hook" }))
      .toHaveAttribute("aria-checked", "false");
    const codexSwitch = screen.getByRole("switch", {
      name: "Configure Codex live tracing hook",
    });
    expect(codexSwitch).toHaveAttribute("aria-checked", "true");
    expect(
      screen.getByText(
        "Codex trust is managed by Codex. Review the hook in /hooks before it can run.",
      ),
    ).toBeInTheDocument();
    expect(section).not.toHaveTextContent(/\bactive\b/i);

    fireEvent.click(screen.getByRole("switch", { name: "Configure Claude live tracing hook" }));
    expect(onToggle).toHaveBeenCalledWith("claude", true);
  });

  it("keeps switches disabled while status is loading or a provider is updating", () => {
    renderSection({ loading: true });

    expect(screen.getByRole("status")).toHaveTextContent("Loading live tracing…");
    expect(screen.getByRole("switch", { name: "Configure Claude live tracing hook" })).toBeDisabled();
    expect(screen.getByRole("switch", { name: "Configure Codex live tracing hook" })).toBeDisabled();
  });

  it("shows a native error without removing the existing provider rows", () => {
    renderSection({ error: new Error("hook_config_write"), updatingProvider: "claude" });

    expect(screen.getByRole("alert")).toHaveTextContent(
      "Could not update the agent tracing hook.",
    );
    expect(screen.getByRole("switch", { name: "Configure Claude live tracing hook" })).toBeDisabled();
    expect(screen.getByRole("switch", { name: "Configure Codex live tracing hook" })).not.toBeDisabled();
    expect(screen.getAllByRole("switch")).toHaveLength(2);
  });
});
