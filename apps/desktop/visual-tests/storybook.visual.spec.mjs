import { expect, test } from 'playwright/test';
import { cases } from './storybook.visual.cases.mjs';
import { assertPreviewQuality } from './preview-quality.mjs';

for (const visualCase of cases) {
  test(`${visualCase.name} renders without clipping or runtime errors`, async ({ page }) => {
    const runtimeErrors = [];
    page.on('pageerror', (error) => runtimeErrors.push(error.message));
    page.on('console', (message) => {
      if (message.type() === 'error') runtimeErrors.push(message.text());
    });

    await page.setViewportSize(visualCase.viewport);
    if (visualCase.reducedMotion) {
      await page.emulateMedia({ reducedMotion: visualCase.reducedMotion });
    }
    const query = new URLSearchParams({
      id: visualCase.story,
      viewMode: 'story',
      globals: `colorTheme:${visualCase.theme}`,
    });
    await page.goto(`/iframe.html?${query}`);

    await page.evaluate(() => document.fonts.ready);
    await visualCase.validate(page);
    await assertPreviewQuality(page, visualCase.theme, runtimeErrors);

    await expect(page).toHaveScreenshot(`${visualCase.name}.png`, { fullPage: true });
  });
}
