export type ThemeSetting = "auto" | "light" | "dark";

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
