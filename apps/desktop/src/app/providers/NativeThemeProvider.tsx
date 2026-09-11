import type { ReactNode } from 'react';
import { ThemeProvider, type ThemePersistence } from '@/shared/theme';
import { loadStoredTheme, saveStoredTheme, subscribeStoredTheme } from './themePersistence';
const persistence: ThemePersistence = {
  load: loadStoredTheme,
  save: saveStoredTheme,
  subscribe: subscribeStoredTheme,
};
export function NativeThemeProvider({ children }: { children: ReactNode }) {
  return <ThemeProvider persistence={persistence}>{children}</ThemeProvider>;
}
