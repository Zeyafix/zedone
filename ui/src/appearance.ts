export type Theme = "system" | "light" | "dark";
export const themeKey = "zedone.appearance.v1";
export function readTheme(): Theme {
  try {
    const value = localStorage.getItem(themeKey);
    if (value === "light" || value === "dark") return value;
  } catch { /* Preferences must not prevent startup. */ }
  return "system";
}
export function saveTheme(theme: Theme): boolean {
  try {
    if (theme === "system") localStorage.removeItem(themeKey);
    else localStorage.setItem(themeKey, theme);
    return true;
  } catch { return false; }
}
