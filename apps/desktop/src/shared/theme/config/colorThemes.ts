export const COLOR_THEMES = [
  {
    id: 'auralis',
    label: 'Auralis Mint',
    description: 'Основная графитовая палитра с мятным акцентом.',
    appearance: 'dark',
  },
  {
    id: 'abyss',
    label: 'Abyss Cyan',
    description: 'Глубокая тёмная cyan/navy палитра.',
    appearance: 'dark',
  },
  {
    id: 'indigo',
    label: 'Indigo Night',
    description: 'Тёмная indigo-палитра с холодным AI-характером.',
    appearance: 'dark',
  },
  {
    id: 'ember',
    label: 'Ember Noir',
    description: 'Тёплая тёмная палитра с коралловым акцентом.',
    appearance: 'dark',
  },
  {
    id: 'violet',
    label: 'Violet Signal',
    description: 'Альтернативная тёмная палитра с фиолетовым акцентом.',
    appearance: 'dark',
  },
  {
    id: 'frost',
    label: 'Auralis Frost',
    description: 'Светлая фирменная палитра с мятным акцентом.',
    appearance: 'light',
  },
  {
    id: 'polar',
    label: 'Polar Blue',
    description: 'Холодная светлая палитра с синим акцентом.',
    appearance: 'light',
  },
  {
    id: 'sandstone',
    label: 'Sandstone',
    description: 'Тёплая светлая палитра с терракотовым акцентом.',
    appearance: 'light',
  },
] as const;

export type ColorTheme = (typeof COLOR_THEMES)[number]['id'];

export const DEFAULT_COLOR_THEME: ColorTheme = 'auralis';

export function isColorTheme(value: unknown): value is ColorTheme {
  return COLOR_THEMES.some((theme) => theme.id === value);
}
