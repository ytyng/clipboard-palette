export type ThemeSetting = "auto" | "light" | "dark";

declare global {
  interface Window {
    __CLIPBOARD_PALETTE_THEME__?: string;
  }
}

const darkMediaQuery = "(prefers-color-scheme: dark)";

let mediaQueryList: MediaQueryList | null = null;
let mediaQueryListener: ((event: MediaQueryListEvent) => void) | null = null;

function setDocumentTheme(theme: "light" | "dark") {
  document.documentElement.dataset.theme = theme;
}

function stopWatchingSystemTheme() {
  if (mediaQueryList && mediaQueryListener) {
    mediaQueryList.removeEventListener("change", mediaQueryListener);
  }
  mediaQueryList = null;
  mediaQueryListener = null;
}

/**
 * The theme injected by the Tauri initialization script (src-tauri/src/lib.rs).
 * Falls back to auto when the page is not running inside the app.
 */
export function injectedTheme(): ThemeSetting {
  const forced = window.__CLIPBOARD_PALETTE_THEME__;
  return forced === "light" || forced === "dark" ? forced : "auto";
}

/**
 * Apply a theme setting.
 * With auto, follow the OS setting and keep tracking later changes.
 */
export function applyTheme(setting: ThemeSetting) {
  stopWatchingSystemTheme();

  if (setting === "light" || setting === "dark") {
    setDocumentTheme(setting);
    return;
  }

  mediaQueryList = window.matchMedia(darkMediaQuery);
  setDocumentTheme(mediaQueryList.matches ? "dark" : "light");
  mediaQueryListener = (event) => setDocumentTheme(event.matches ? "dark" : "light");
  mediaQueryList.addEventListener("change", mediaQueryListener);
}
