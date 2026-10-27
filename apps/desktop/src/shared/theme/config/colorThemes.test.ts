import { describe, expect, it } from 'vitest';
import { COLOR_THEMES, DEFAULT_COLOR_THEME, isColorTheme, migrateColorTheme } from './colorThemes';

describe('color theme configuration', () => {
  it('registers the complete theme collection', () => {
    expect(COLOR_THEMES.map(({ id }) => id)).toEqual(['dark', 'light']);
  });

  it('recognizes registered themes and rejects unsupported values', () => {
    expect(DEFAULT_COLOR_THEME).toBe('dark');
    expect(isColorTheme('light')).toBe(true);
    expect(isColorTheme('frost')).toBe(false);
    expect(isColorTheme('unknown')).toBe(false);
    expect(isColorTheme(null)).toBe(false);
  });

  it('classifies themes by their visual appearance', () => {
    expect(
      COLOR_THEMES.filter(({ appearance }) => appearance === 'light').map(({ id }) => id),
    ).toEqual(['light']);
    expect(
      COLOR_THEMES.filter(({ appearance }) => appearance === 'dark').map(({ id }) => id),
    ).toEqual(['dark']);
  });
  it('migrates every removed palette without exposing it as a selectable theme', () => {
    for (const value of ['auralis', 'abyss', 'indigo', 'ember', 'violet'])
      expect(migrateColorTheme(value)).toBe('dark');
    for (const value of ['frost', 'polar', 'sandstone'])
      expect(migrateColorTheme(value)).toBe('light');
    expect(migrateColorTheme('future-theme')).toBeUndefined();
  });
});
