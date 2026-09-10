import type { Preview } from '@storybook/react-vite';
import '../src/app/styles/index.css';
import {
  COLOR_THEMES,
  DEFAULT_COLOR_THEME,
  isColorTheme,
} from '../src/shared/theme/config/colorThemes';
import { auralisStorybookTheme } from './auralisTheme';
import { installTauriStoryAdapter } from './tauriStoryAdapter';

installTauriStoryAdapter();

const preview: Preview = {
  globalTypes: {
    colorTheme: {
      name: 'Auralis theme',
      description: 'Semantic color theme used by the rendered product interface',
      toolbar: {
        icon: 'paintbrush',
        dynamicTitle: true,
        items: COLOR_THEMES.map((theme) => ({
          value: theme.id,
          title: theme.label,
          right: theme.appearance === 'dark' ? 'Dark' : 'Light',
        })),
      },
    },
  },
  initialGlobals: {
    colorTheme: DEFAULT_COLOR_THEME,
  },
  decorators: [
    (Story, context) => {
      const requestedTheme = context.globals.colorTheme;
      const colorTheme = isColorTheme(requestedTheme) ? requestedTheme : DEFAULT_COLOR_THEME;
      const appearance =
        COLOR_THEMES.find((theme) => theme.id === colorTheme)?.appearance ?? 'dark';

      document.documentElement.dataset.colorTheme = colorTheme;
      document.documentElement.style.colorScheme = appearance;

      return (
        <div data-storybook-color-theme={colorTheme} style={{ minHeight: '100%' }}>
          <Story />
        </div>
      );
    },
  ],
  parameters: {
    controls: {
      disable: true,
      expanded: true,
      sort: 'requiredFirst',
      matchers: {
        color: /(background|color)$/i,
        date: /Date$/i,
      },
    },

    a11y: {
      test: 'error',
    },
    docs: {
      theme: auralisStorybookTheme,
      toc: true,
      source: {
        type: 'dynamic',
      },
    },
    options: {
      storySort: {
        order: [
          'Design System',
          ['Introduction', 'Foundations', 'Components'],
          'Product',
          ['Pages', 'Features', 'Widgets'],
        ],
      },
    },
    viewport: {
      options: {
        desktop1440: {
          name: 'Desktop · 1440 × 900',
          styles: { width: '1440px', height: '900px' },
        },
        desktop1280: {
          name: 'Desktop · 1280 × 800',
          styles: { width: '1280px', height: '800px' },
        },
        tablet1024: {
          name: 'Tablet · 1024 × 768',
          styles: { width: '1024px', height: '768px' },
        },
        compact800: {
          name: 'Compact · 800 × 700',
          styles: { width: '800px', height: '700px' },
        },
      },
    },
  },
};

export default preview;
