import { useContext } from 'react';
import { ThemeContext } from './ThemeContext';

export function useColorTheme() {
  const context = useContext(ThemeContext);
  if (!context) throw new Error('useColorTheme must be used inside ThemeProvider');
  return context;
}
