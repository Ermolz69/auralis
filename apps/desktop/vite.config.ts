/// <reference types="vitest/config" />
import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import tailwindcss from '@tailwindcss/vite';
import path from 'path';
import { fileURLToPath } from 'url';
import { storybookTest } from '@storybook/addon-vitest/vitest-plugin';
import { playwright } from '@vitest/browser-playwright';
import { readFileSync } from 'node:fs';
import { bundleReport } from './build/bundleReport.js';
const dirname =
  typeof __dirname !== 'undefined' ? __dirname : path.dirname(fileURLToPath(import.meta.url));
const desktopPackage = JSON.parse(
  readFileSync(new URL('./package.json', import.meta.url), 'utf8'),
) as {
  version: string;
};
const nativeE2e = process.env.AURALIS_NATIVE_E2E === '1';
const nativeE2eMediaPath = nativeE2e ? process.env.AURALIS_NATIVE_E2E_MEDIA_PATH : '';
const nativeE2eRunId = nativeE2e ? process.env.AURALIS_NATIVE_E2E_RUN_ID : '';
const nativeE2eYtdlpUrl = nativeE2e ? process.env.AURALIS_NATIVE_E2E_YTDLP_URL : '';

if (nativeE2e && (!nativeE2eMediaPath || !nativeE2eRunId || !nativeE2eYtdlpUrl)) {
  throw new Error(
    'AURALIS_NATIVE_E2E_MEDIA_PATH, AURALIS_NATIVE_E2E_RUN_ID and AURALIS_NATIVE_E2E_YTDLP_URL are required for native E2E builds',
  );
}

// More info at: https://storybook.js.org/docs/next/writing-tests/integrations/vitest-addon
const __vite_dirname = path.dirname(fileURLToPath(import.meta.url));

// https://vite.dev/config/
export default defineConfig({
  define: {
    __APP_VERSION__: JSON.stringify(desktopPackage.version),
    __NATIVE_E2E__: JSON.stringify(nativeE2e),
    __NATIVE_E2E_MEDIA_PATH__: JSON.stringify(nativeE2eMediaPath ?? ''),
    __NATIVE_E2E_RUN_ID__: JSON.stringify(nativeE2eRunId ?? ''),
    __NATIVE_E2E_YTDLP_URL__: JSON.stringify(nativeE2eYtdlpUrl ?? ''),
  },
  plugins: [react(), tailwindcss(), bundleReport()],
  server: { port: 5173, strictPort: true },
  build: {
    outDir: nativeE2e ? 'dist-native-e2e' : 'dist',
    manifest: true,
    rolldownOptions: {
      output: {
        codeSplitting: {
          groups: [{ name: 'vendor', test: /node_modules/ }],
        },
      },
    },
  },
  resolve: {
    alias: {
      '@': path.resolve(__vite_dirname, './src'),
    },
  },
  test: {
    coverage: {
      provider: 'v8',
      reporter: ['text', 'json-summary', 'lcov'],
      reportsDirectory: './coverage',
      include: ['src/**/*.{ts,tsx}'],
      exclude: [
        '.storybook/**',
        'src/**/*.test.{ts,tsx}',
        'src/**/*.stories.{ts,tsx}',
        'src/**/*.storyData.ts',
        'src/**/*.storyFixtures.tsx',
        'src/app/native-e2e/**',
        'src/**/*.d.ts',
        'src/**/index.ts',
        'src/main.tsx',
      ],
      thresholds: {
        statements: 90,
        branches: 80,
        functions: 90,
        lines: 92,
      },
    },
    projects: [
      {
        extends: true,
        test: {
          name: 'unit',
          include: ['src/**/*.test.ts'],
        },
      },
      {
        extends: true,
        test: {
          name: 'component',
          environment: 'jsdom',
          include: ['src/**/*.test.tsx'],
          exclude: ['src/**/*.integration.test.tsx', 'src/App.selection.test.tsx'],
        },
      },
      {
        extends: true,
        test: {
          name: 'integration',
          environment: 'jsdom',
          include: ['src/**/*.integration.test.tsx', 'src/App.selection.test.tsx'],
        },
      },
      {
        extends: true,
        plugins: [
          // The plugin will run tests for the stories defined in your Storybook config
          // See options at: https://storybook.js.org/docs/next/writing-tests/integrations/vitest-addon#storybooktest
          storybookTest({
            configDir: path.join(dirname, '.storybook'),
          }),
        ],
        test: {
          name: 'storybook',
          browser: {
            enabled: true,
            headless: true,
            provider: playwright({}),
            instances: [
              {
                browser: 'chromium',
              },
            ],
          },
        },
      },
    ],
  },
});
