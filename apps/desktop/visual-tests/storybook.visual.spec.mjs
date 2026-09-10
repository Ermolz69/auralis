import { expect, test } from 'playwright/test';

const cases = [
  {
    name: 'control-safety-dark-wide',
    story: 'design-system-foundations-control-safety-matrix--all-allowed-values',
    theme: 'auralis',
    viewport: { width: 1440, height: 900 },
    validate: async (page) => {
      await expect(page.getByTestId('safe-button')).toHaveCount(14);
      await expect(page.getByTestId('safe-select')).toHaveCount(2);
    },
  },
  {
    name: 'control-safety-light-compact',
    story: 'design-system-foundations-control-safety-matrix--all-allowed-values',
    theme: 'frost',
    viewport: { width: 800, height: 700 },
    validate: async (page) => {
      await expect(page.getByTestId('safe-icon').first()).toBeVisible();
      await expect(page.getByLabel('Large finite option list')).toHaveValue('0');
    },
  },
  {
    name: 'all-themes-gallery',
    story: 'design-system-foundations-theme-gallery--all-themes',
    theme: 'auralis',
    viewport: { width: 1440, height: 900 },
  },
  {
    name: 'long-project-title-wide',
    story: 'product-widgets-app-shell--with-long-project-title',
    theme: 'auralis',
    viewport: { width: 1440, height: 900 },
  },
  {
    name: 'long-project-title-compact-light',
    story: 'product-widgets-app-shell--with-long-project-title',
    theme: 'frost',
    viewport: { width: 800, height: 700 },
  },
  {
    name: 'home-empty',
    story: 'product-pages-home-states--local-first-empty',
    theme: 'auralis',
    viewport: { width: 1280, height: 800 },
  },
  {
    name: 'home-loading',
    story: 'product-pages-home-states--loading-recent-projects',
    theme: 'frost',
    viewport: { width: 1024, height: 768 },
  },
  {
    name: 'project-list-error',
    story: 'product-features-project-list-states--fetch-error',
    theme: 'auralis',
    viewport: { width: 800, height: 700 },
  },
  {
    name: 'transcript-waiting',
    story: 'product-widgets-transcript-editor-states--waiting-for-pipeline',
    theme: 'frost',
    viewport: { width: 1280, height: 800 },
  },
  {
    name: 'transcript-loading',
    story: 'product-widgets-transcript-editor-states--loading-transcript',
    theme: 'auralis',
    viewport: { width: 1024, height: 768 },
  },
  {
    name: 'transcript-error',
    story: 'product-widgets-transcript-editor-states--error-with-retry',
    theme: 'frost',
    viewport: { width: 800, height: 700 },
  },
  {
    name: 'job-history-100-items',
    story: 'product-widgets-job-queue-panel-states--large-history',
    theme: 'auralis',
    viewport: { width: 1024, height: 768 },
    validate: async (page) => {
      await expect(page.getByRole('listitem')).toHaveCount(100);
      const history = page.getByRole('list', { name: 'Operation history' });
      await expect(history).toBeVisible();
      expect(
        await history.evaluate((element) => {
          const scrollParent = element.parentElement?.parentElement;
          return Boolean(scrollParent && scrollParent.scrollHeight > scrollParent.clientHeight);
        }),
      ).toBe(true);
    },
  },
  {
    name: 'job-history-100-items-oldest-visible',
    story: 'product-widgets-job-queue-panel-states--large-history',
    theme: 'frost',
    viewport: { width: 800, height: 700 },
    validate: async (page) => {
      const oldestJob = page.getByRole('listitem').last();
      await oldestJob.scrollIntoViewIfNeeded();
      await expect(oldestJob).toContainText('Transcript generation 001');
      await expect(oldestJob).toBeVisible();
    },
  },
];

for (const visualCase of cases) {
  test(`${visualCase.name} renders without clipping or runtime errors`, async ({ page }) => {
    const runtimeErrors = [];
    page.on('pageerror', (error) => runtimeErrors.push(error.message));
    page.on('console', (message) => {
      if (message.type() === 'error') runtimeErrors.push(message.text());
    });

    await page.setViewportSize(visualCase.viewport);
    const query = new URLSearchParams({
      id: visualCase.story,
      viewMode: 'story',
      globals: `colorTheme:${visualCase.theme}`,
    });
    await page.goto(`/iframe.html?${query}`);

    const storyRoot = page.locator('#storybook-root');
    await expect(storyRoot).not.toBeEmpty();
    await page.evaluate(() => document.fonts.ready);
    await expect(page.locator('.sb-errordisplay:visible')).toHaveCount(0);
    await visualCase.validate?.(page);

    const pageOverflow = await page.evaluate(() => ({
      viewportWidth: window.innerWidth,
      contentWidth: Math.max(document.documentElement.scrollWidth, document.body.scrollWidth),
    }));
    expect(pageOverflow.contentWidth).toBeLessThanOrEqual(pageOverflow.viewportWidth + 1);
    expect(runtimeErrors).toEqual([]);

    await expect(page).toHaveScreenshot(`${visualCase.name}.png`, { fullPage: true });
  });
}
