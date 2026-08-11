import { describe, expect, it } from 'vitest';
import { COLOR_THEMES, DEFAULT_COLOR_THEME, isColorTheme } from './colorThemes';

describe('color theme configuration', () => {
  it('registers the complete theme collection', () => {
    expect(COLOR_THEMES.map(({ id }) => id)).toEqual([
      'auralis',
      'abyss',
      'indigo',
      'ember',
      'violet',
      'frost',
      'polar',
      'sandstone',
    ]);
  });

  it('recognizes registered themes and rejects unsupported values', () => {
    expect(DEFAULT_COLOR_THEME).toBe('auralis');
    expect(isColorTheme('frost')).toBe(true);
    expect(isColorTheme('unknown')).toBe(false);
    expect(isColorTheme(null)).toBe(false);
  });

  it('classifies themes by their visual appearance', () => {
    expect(
      COLOR_THEMES.filter(({ appearance }) => appearance === 'light').map(({ id }) => id),
    ).toEqual(['frost', 'polar', 'sandstone']);
    expect(
      COLOR_THEMES.filter(({ appearance }) => appearance === 'dark').map(({ id }) => id),
    ).toEqual(['auralis', 'abyss', 'indigo', 'ember', 'violet']);
  });
});
