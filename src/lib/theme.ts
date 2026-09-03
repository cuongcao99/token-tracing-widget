export const themeRegistry = [{ id: "claude", label: "Claude" }] as const;

export type ThemeId = (typeof themeRegistry)[number]["id"];

export const themeOrder = themeRegistry.map(({ id }) => id) as ThemeId[];

export function isThemeId(value: unknown): value is ThemeId {
  return (
    typeof value === "string" &&
    themeRegistry.some((theme) => theme.id === value)
  );
}
