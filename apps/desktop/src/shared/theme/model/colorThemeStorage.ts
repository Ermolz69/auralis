import { DEFAULT_COLOR_THEME, migrateColorTheme, type ColorTheme } from '../config/colorThemes';

export const COLOR_THEME_STORAGE_KEY = 'auralis:color-theme:v1';

export type StoredColorTheme =
  | { status: 'valid'; theme: ColorTheme; storedValue: string }
  | { status: 'missing' | 'invalid' | 'unavailable' };

export function readStoredColorTheme(): StoredColorTheme {
  try {
    if (!globalThis.localStorage) return { status: 'unavailable' };
    const storedTheme = globalThis.localStorage.getItem(COLOR_THEME_STORAGE_KEY);
    if (storedTheme === null) return { status: 'missing' };
    const theme = migrateColorTheme(storedTheme);
    return theme ? { status: 'valid', theme, storedValue: storedTheme } : { status: 'invalid' };
  } catch {
    return { status: 'unavailable' };
  }
}

export function getStoredColorTheme(): ColorTheme {
  const stored = readStoredColorTheme();
  return stored.status === 'valid' ? stored.theme : DEFAULT_COLOR_THEME;
}

export function applyColorTheme(theme: ColorTheme): void {
  globalThis.document?.documentElement.setAttribute('data-color-theme', theme);
}

export function persistColorTheme(theme: ColorTheme): boolean {
  try {
    if (!globalThis.localStorage) return false;
    globalThis.localStorage.setItem(COLOR_THEME_STORAGE_KEY, theme);
    return true;
  } catch {
    return false;
  }
}

export function initializeColorTheme(): ColorTheme {
  const theme = DEFAULT_COLOR_THEME;
  applyColorTheme(theme);
  return theme;
}
