import { addons } from 'storybook/manager-api';
import { auralisStorybookTheme } from './auralisTheme';

addons.setConfig({
  theme: auralisStorybookTheme,
  navSize: 300,
  bottomPanelHeight: 320,
  panelPosition: 'bottom',
  enableShortcuts: true,
  showToolbar: true,
  sidebar: {
    showRoots: true,
  },
  toolbar: {
    title: { hidden: false },
    zoom: { hidden: false },
    eject: { hidden: false },
    copy: { hidden: false },
    fullscreen: { hidden: false },
  },
});
