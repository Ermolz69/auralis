import { defineConfig, devices } from 'playwright/test';

const storybookPort = 6007;

export default defineConfig({
  testDir: './visual-tests',
  outputDir: './storybook-visual-results/test-artifacts',
  globalSetup: './visual-tests/storybook.visual.setup.mjs',
  fullyParallel: false,
  workers: 1,
  retries: process.env.CI ? 1 : 0,
  timeout: 30_000,
  expect: {
    timeout: 10_000,
    toHaveScreenshot: {
      animations: 'disabled',
      caret: 'hide',
      scale: 'css',
      threshold: 0.2,
      maxDiffPixelRatio: 0.005,
    },
  },
  use: {
    ...devices['Desktop Chrome'],
    baseURL: `http://127.0.0.1:${storybookPort}`,
    browserName: 'chromium',
    colorScheme: 'dark',
    locale: 'en-US',
    timezoneId: 'UTC',
    reducedMotion: 'reduce',
    deviceScaleFactor: 1,
    screenshot: 'only-on-failure',
    trace: 'retain-on-failure',
  },
  reporter: process.env.CI
    ? [['line'], ['junit', { outputFile: 'storybook-visual-results/junit.xml' }]]
    : [['list'], ['html', { outputFolder: 'storybook-visual-results/report', open: 'never' }]],
  snapshotPathTemplate: '{testDir}/__snapshots__/{arg}-{projectName}-{platform}{ext}',
});
