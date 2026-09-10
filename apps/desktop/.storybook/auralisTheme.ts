import { create } from 'storybook/theming';

export const auralisStorybookTheme = create({
  base: 'dark',
  brandTitle: 'Auralis Design System',
  brandUrl: './',
  brandTarget: '_self',

  colorPrimary: '#4de2c1',
  colorSecondary: '#75a7ff',

  appBg: '#090c0f',
  appContentBg: '#0d1116',
  appPreviewBg: '#090c0f',
  appBorderColor: '#28333d',
  appBorderRadius: 9,

  fontBase: "Inter, 'Segoe UI Variable', 'Segoe UI', sans-serif",
  fontCode: "'JetBrains Mono', 'Cascadia Code', Consolas, monospace",

  textColor: '#f1f5f7',
  textInverseColor: '#071411',
  textMutedColor: '#aab5bf',

  barTextColor: '#aab5bf',
  barHoverColor: '#f1f5f7',
  barSelectedColor: '#4de2c1',
  barBg: '#121820',

  buttonBg: '#18212a',
  buttonBorder: '#28333d',
  booleanBg: '#18212a',
  booleanSelectedBg: '#4de2c1',

  inputBg: '#121820',
  inputBorder: '#28333d',
  inputTextColor: '#f1f5f7',
  inputBorderRadius: 6,
});
