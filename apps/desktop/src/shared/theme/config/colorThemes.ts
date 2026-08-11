export const COLOR_THEMES = [
  {
    id: 'auralis',
    label: 'Auralis Mint',
    description: 'Основная графитовая палитра с мятным акцентом.',
  },
  {
    id: 'violet',
    label: 'Violet Signal',
    description: 'Альтернативная тёмная палитра с фиолетовым акцентом.',
  },
] as const;

export type ColorTheme = (typeof COLOR_THEMES)[number]['id'];

export const DEFAULT_COLOR_THEME: ColorTheme = 'auralis';

export function isColorTheme(value: unknown): value is ColorTheme {
  return COLOR_THEMES.some((theme) => theme.id === value);
}
