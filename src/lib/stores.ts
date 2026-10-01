import { writable } from "svelte/store";

export type Screen = "home" | "browse" | "models" | "chat" | "documents" | "settings";

export const currentScreen = writable<Screen>("home");

// ---- Theme ----
// "system" follows the Ubuntu setting. The choice is kept per computer in localStorage.

export type Theme = "system" | "light" | "dark";

const THEME_KEY = "kuttylm.theme";

function loadTheme(): Theme {
  try {
    const saved = localStorage.getItem(THEME_KEY);
    if (saved === "light" || saved === "dark" || saved === "system") return saved;
  } catch {
    // Storage can be blocked; fall back to the system theme.
  }
  return "system";
}

export const theme = writable<Theme>(loadTheme());

theme.subscribe((value) => {
  const root = document.documentElement;
  if (value === "system") root.removeAttribute("data-theme");
  else root.setAttribute("data-theme", value);
  try {
    localStorage.setItem(THEME_KEY, value);
  } catch {
    // Ignore: the theme still works for this session.
  }
});
