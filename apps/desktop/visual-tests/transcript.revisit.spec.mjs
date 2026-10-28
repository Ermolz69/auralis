import { expect, test } from 'playwright/test';

const prefix = 'product-widgets-transcript-editor-states--';
const ready = `${prefix}ready-read-only-transcript`;
const readyText = 'Welcome to the product walkthrough.';

async function selectStory(page, story) {
  const link = page.locator(`a[href*="path=/story/${story}"]`);
  await expect(link).toHaveCount(1);
  await link.click();
  await expect(page).toHaveURL(new RegExp(`path=/story/${story}(?:&|$)`));
}

for (const [name, story, expected] of [
  ['waiting', 'waiting-for-pipeline', 'Waiting for subtitles'],
  ['error', 'error-with-retry', 'Transcript unavailable'],
]) {
  test(`transcript revisit restores ready content after ${name} in the same manager session`, async ({
    page,
  }) => {
    await page.goto(`/?path=/story/${ready}`);
    const preview = page.frameLocator('#storybook-preview-iframe');
    await expect(preview.getByText(readyText)).toBeVisible();
    const documentStarted = await preview.locator('body').evaluate(() => performance.timeOrigin);

    await selectStory(page, `${prefix}${story}`);
    await expect(preview.getByText(expected, { exact: true })).toBeVisible();
    await expect(preview.getByText(readyText)).toHaveCount(0);
    expect(await preview.locator('body').evaluate(() => performance.timeOrigin)).toBe(
      documentStarted,
    );

    await selectStory(page, ready);
    await expect(preview.getByText(readyText)).toBeVisible();
    await expect(preview.getByText(expected, { exact: true })).toHaveCount(0);
    await expect(preview.locator('.sb-errordisplay:visible, .sb-nopreview:visible')).toHaveCount(0);
    expect(await preview.locator('body').evaluate(() => performance.timeOrigin)).toBe(
      documentStarted,
    );
  });
}
