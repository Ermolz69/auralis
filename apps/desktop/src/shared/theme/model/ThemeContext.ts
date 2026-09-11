import { createContext } from 'react';
import type { ColorTheme } from '../config/colorThemes';

export type ThemeContextValue = {
  colorTheme: ColorTheme;
  persistenceStatus: 'idle' | 'pending' | 'saved' | 'error';
  setColorTheme: (theme: ColorTheme) => void;
};

export const ThemeContext = createContext<ThemeContextValue | null>(null);
