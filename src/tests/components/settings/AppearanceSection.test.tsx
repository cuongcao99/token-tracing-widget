import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import AppearanceSection from "../../../components/settings/AppearanceSection";
import themePickerStyles from "../../../styles/settings/theme-picker.module.css";

describe("AppearanceSection", () => {
  it("uses a styled theme picker with a keyboard-accessible option list", () => {
    const onThemeChange = vi.fn();

    render(
      <AppearanceSection
        theme="claude"
        onThemeChange={onThemeChange}
        darkMode={true}
        onToggle={() => undefined}
      />,
    );

    const themePicker = screen.getByRole("button", { name: "Theme: Claude" });

    expect(themePicker).toHaveClass(themePickerStyles.button);
    expect(themePicker).toHaveAttribute("aria-expanded", "false");

    fireEvent.click(themePicker);

    expect(themePicker).toHaveAttribute("aria-expanded", "true");
    expect(screen.getByRole("listbox", { name: "Theme options" })).toBeInTheDocument();
    expect(screen.getByRole("option", { name: "Claude" })).toHaveAttribute(
      "aria-selected",
      "true",
    );

    fireEvent.keyDown(document, { key: "Escape" });
    expect(themePicker).toHaveAttribute("aria-expanded", "false");
    expect(screen.queryByRole("listbox", { name: "Theme options" })).not.toBeInTheDocument();

    fireEvent.click(themePicker);
    fireEvent.click(screen.getByRole("option", { name: "Claude" }));
    expect(onThemeChange).toHaveBeenCalledWith("claude");
  });
});
