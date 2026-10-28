import { expect, test } from 'playwright/test';
import { assertPreviewQuality, assertReachableControl } from './preview-quality.mjs';

async function installPreview(page, extra = '') {
  await page.setContent(`
    <html data-color-theme="dark" style="color-scheme:dark;--color-bg:#101010;--color-text:#eeeeee">
      <body style="background:#101010;color:#eeeeee"><div id="storybook-root"><div data-storybook-color-theme="dark">
        <button>Preview action</button>${extra}
      </div></div></body>
    </html>
  `);
}

test('preview guard accepts a themed, reachable control', async ({ page }) => {
  await installPreview(page);
  await assertPreviewQuality(page, 'dark', []);
  await assertReachableControl(page.getByRole('button', { name: 'Preview action' }));
});

test('preview guard rejects a failed story with nonempty fallback content', async ({ page }) => {
  await installPreview(page, '<div class="sb-errordisplay">Story failed to render</div>');
  await expect(assertPreviewQuality(page, 'dark', [])).rejects.toThrow();
});

test('preview guard rejects the wrong theme and missing theme styles', async ({ page }) => {
  await installPreview(page);
  await expect(assertPreviewQuality(page, 'light', [])).rejects.toThrow();
  await page.locator('html').evaluate((element) => element.style.removeProperty('--color-bg'));
  await expect(assertPreviewQuality(page, 'dark', [])).rejects.toThrow();
});

test('preview guard rejects runtime errors and horizontal overflow', async ({ page }) => {
  await installPreview(page);
  await expect(assertPreviewQuality(page, 'dark', ['Unhandled API response'])).rejects.toThrow();
  await page.locator('#storybook-root').evaluate((element) => {
    element.style.width = '200vw';
  });
  await expect(assertPreviewQuality(page, 'dark', [])).rejects.toThrow();
});

test('preview guard rejects an incorrect painted theme despite correct tokens', async ({
  page,
}) => {
  await installPreview(page);
  await page.locator('body').evaluate((element) => {
    element.style.backgroundColor = 'white';
  });
  await expect(assertPreviewQuality(page, 'dark', [])).rejects.toThrow();
});

test('visible controls must receive pointer input through overlays', async ({ page }) => {
  await installPreview(page, '<div style="position:fixed;inset:0;z-index:2"></div>');
  const button = page.getByRole('button', { name: 'Preview action' });
  await expect(button).toBeVisible();
  await expect(assertReachableControl(button)).rejects.toThrow();
});

test('visible controls clipped by inner containers are rejected', async ({ page }) => {
  await installPreview(page);
  await page.locator('#storybook-root').evaluate((element) => {
    element.style.cssText = 'width:120px;overflow:hidden';
    element.querySelector('button').style.width = '200px';
  });
  await expect(assertReachableControl(page.getByRole('button'))).rejects.toThrow();
});

test('offscreen controls inside a scrollable panel remain reachable', async ({ page }) => {
  await installPreview(
    page,
    `
    <div style="height:80px;overflow:auto">
      <div style="height:500px"></div><button>Oldest result</button>
    </div>
  `,
  );
  await assertReachableControl(page.getByRole('button', { name: 'Oldest result' }));
});
