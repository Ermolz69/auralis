import type { StorybookConfig } from '@storybook/react-vite';

const config: StorybookConfig = {
  stories: ['../src/**/*.mdx', '../src/**/*.stories.@(js|jsx|mjs|ts|tsx)'],
  addons: [
    '@chromatic-com/storybook',
    '@storybook/addon-vitest',
    '@storybook/addon-a11y',
    '@storybook/addon-docs',
    '@storybook/addon-mcp',
  ],
  framework: '@storybook/react-vite',
  docs: {
    defaultName: 'Documentation',
  },
  core: {
    disableTelemetry: true,
  },
  features: {
    menuOnboardingChecklist: false,
    sidebarOnboardingChecklist: false,
  },
  async viteFinal(viteConfig) {
    const buildOptions = viteConfig.build;
    const rolldownOptions = buildOptions?.rolldownOptions;

    return {
      ...viteConfig,
      plugins: viteConfig.plugins?.filter(
        (plugin) =>
          !plugin ||
          typeof plugin !== 'object' ||
          Array.isArray(plugin) ||
          !('name' in plugin) ||
          plugin.name !== 'auralis-bundle-report',
      ),
      build: {
        ...buildOptions,
        // Storybook bundles its documentation runtime and accessibility engine into the preview.
        // Keep the product bundle budget independent from this development-only catalog.
        chunkSizeWarningLimit: 1_200,
        rolldownOptions: {
          ...rolldownOptions,
          checks: {
            ...rolldownOptions?.checks,
            pluginTimings: false,
          },
        },
      },
    };
  },
};
export default config;
