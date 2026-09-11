import type { ColorTheme } from '../config/colorThemes';
export type ThemeSnapshot = { value: string; revision: number };
export interface ThemePersistence {
  load(): Promise<ThemeSnapshot | null>;
  save(value: ColorTheme): Promise<ThemeSnapshot>;
  subscribe(listener: (value: ThemeSnapshot) => void): () => void;
}
