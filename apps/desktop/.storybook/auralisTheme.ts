import { create } from 'storybook/theming';
import tokens from '../src/shared/theme/config/tesseraTokens.json';

const palette = tokens.themes.dark;
export const auralisStorybookTheme = create({
  base: 'dark',
  brandTitle: 'Auralis Design System',
  brandUrl: './',
  brandTarget: '_self',
  colorPrimary: palette.primary,
  colorSecondary: palette.accent,
  appBg: palette.canvas,
  appContentBg: palette.panel,
  appPreviewBg: palette.canvas,
  appBorderColor: palette.border,
  appBorderRadius: tokens.shape.cornerRadius,
  fontBase: tokens.type.ui,
  fontCode: tokens.type.technical,
  textColor: palette.text,
  textInverseColor: palette.primaryText,
  textMutedColor: palette.textSecondary,
  barTextColor: palette.textSecondary,
  barHoverColor: palette.text,
  barSelectedColor: palette.primary,
  barBg: palette.raised,
  buttonBg: palette.panel,
  buttonBorder: palette.border,
  booleanBg: palette.panel,
  booleanSelectedBg: palette.primary,
  inputBg: palette.raised,
  inputBorder: palette.border,
  inputTextColor: palette.text,
  inputBorderRadius: tokens.shape.cornerRadius,
});
