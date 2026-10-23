export const COLOR_THEMES = [
  {
    id: 'dark',
    label: 'Tessera Dark',
    description: 'Тёмная монохромная тема Tessera.',
    appearance: 'dark',
  },
  {
    id: 'light',
    label: 'Tessera Light',
    description: 'Светлая тема Tessera с тёплым фоном.',
    appearance: 'light',
  },
] as const;

export type ColorTheme = (typeof COLOR_THEMES)[number]['id'];
export const DEFAULT_COLOR_THEME: ColorTheme = 'dark';

export function isColorTheme(value: unknown): value is ColorTheme {
  return COLOR_THEMES.some((theme) => theme.id === value);
}

export function migrateColorTheme(value: unknown): ColorTheme | undefined {
  if (isColorTheme(value)) return value;
  if (['auralis', 'abyss', 'indigo', 'ember', 'violet'].includes(String(value))) return 'dark';
  if (['frost', 'polar', 'sandstone'].includes(String(value))) return 'light';
  return undefined;
}
