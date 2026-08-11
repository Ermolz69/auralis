import { useCallback, useLayoutEffect, useMemo, useState, type ReactNode } from 'react';
import type { ColorTheme } from '../config/colorThemes';
import {
  applyColorTheme,
  getStoredColorTheme,
  persistColorTheme,
} from './colorThemeStorage';
import { ThemeContext } from './ThemeContext';

export function ThemeProvider({ children }: { children: ReactNode }) {
  const [colorTheme, setColorThemeState] = useState(getStoredColorTheme);

  useLayoutEffect(() => {
    applyColorTheme(colorTheme);
    persistColorTheme(colorTheme);
  }, [colorTheme]);

  const setColorTheme = useCallback((theme: ColorTheme) => {
    setColorThemeState(theme);
  }, []);

  const value = useMemo(() => ({ colorTheme, setColorTheme }), [colorTheme, setColorTheme]);

  return <ThemeContext.Provider value={value}>{children}</ThemeContext.Provider>;
}
