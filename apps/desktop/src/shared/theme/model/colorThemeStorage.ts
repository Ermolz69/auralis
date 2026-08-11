import { DEFAULT_COLOR_THEME, isColorTheme, type ColorTheme } from '../config/colorThemes';

const COLOR_THEME_STORAGE_KEY = 'auralis:color-theme:v1';

export function getStoredColorTheme(): ColorTheme {
  try {
    const storedTheme = globalThis.localStorage?.getItem(COLOR_THEME_STORAGE_KEY);
    return isColorTheme(storedTheme) ? storedTheme : DEFAULT_COLOR_THEME;
  } catch {
    return DEFAULT_COLOR_THEME;
  }
}

export function applyColorTheme(theme: ColorTheme): void {
  globalThis.document?.documentElement.setAttribute('data-color-theme', theme);
}

export function persistColorTheme(theme: ColorTheme): void {
  try {
    globalThis.localStorage?.setItem(COLOR_THEME_STORAGE_KEY, theme);
  } catch {
    // Theme switching remains functional when persistent WebView storage is unavailable.
  }
}

export function initializeColorTheme(): ColorTheme {
  const theme = getStoredColorTheme();
  applyColorTheme(theme);
  return theme;
}
