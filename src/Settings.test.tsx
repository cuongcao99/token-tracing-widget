import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import Settings from "./Settings";
import {
  getSourceSettings,
  updateSourceSettings,
} from "./lib/source-settings";

vi.mock("./lib/source-settings", () => ({
  getSourceSettings: vi.fn(),
  updateSourceSettings: vi.fn(),
}));

const validSnapshot = {
  sources: [
    { provider: "claude" as const, enabled: true, rootOverride: null },
    { provider: "codex" as const, enabled: false, rootOverride: "C:\\codex" },
  ],
};

beforeEach(() => {
  vi.mocked(getSourceSettings).mockResolvedValue(validSnapshot);
  vi.mocked(updateSourceSettings).mockResolvedValue(validSnapshot);
});

afterEach(() => {
  cleanup();
  vi.clearAllMocks();
});

it("renders both persisted provider settings", async () => {
  render(<Settings />);

  expect(
    await screen.findByRole("heading", { name: "Source settings" }),
  ).toBeInTheDocument();
  expect(
    screen.getByRole("checkbox", { name: "Collect Claude Code" }),
  ).toBeChecked();
  expect(
    screen.getByRole("checkbox", { name: "Collect Codex" }),
  ).not.toBeChecked();
  expect(
    screen.getByRole("textbox", { name: "Codex source root" }),
  ).toHaveValue("C:\\codex");
});

it("saves the changed provider settings and reports success", async () => {
  render(<Settings />);
  await screen.findByRole("heading", { name: "Source settings" });
  fireEvent.click(
    screen.getByRole("checkbox", { name: "Collect Claude Code" }),
  );
  fireEvent.click(screen.getByRole("button", { name: "Save changes" }));

  await waitFor(() => expect(updateSourceSettings).toHaveBeenCalled());
  expect(updateSourceSettings).toHaveBeenNthCalledWith(1, {
    provider: "claude",
    enabled: false,
    rootOverride: null,
  });
  expect(updateSourceSettings).toHaveBeenNthCalledWith(2, {
    provider: "codex",
    enabled: false,
    rootOverride: "C:\\codex",
  });
  expect(await screen.findByRole("status")).toHaveTextContent("Saved");
});

it("shows a sanitized error when saving is rejected", async () => {
  vi.mocked(updateSourceSettings).mockRejectedValue(
    new Error("invalid_root:unsupported_unc:\\server\\private\\sessions"),
  );

  render(<Settings />);
  await screen.findByRole("heading", { name: "Source settings" });
  fireEvent.click(screen.getByRole("button", { name: "Save changes" }));

  expect(await screen.findByRole("alert")).toHaveTextContent(
    "Invalid source root",
  );
  expect(screen.getByRole("alert")).not.toHaveTextContent("\\server");
});
