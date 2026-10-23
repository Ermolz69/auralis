import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import path from 'node:path';
import { test } from 'node:test';
import { chromium } from 'playwright';
import { serveProduction } from './production-server.mjs';
import { installFixture } from './production-fixture.mjs';

function installThemeFixture() {
  const base = window.__TAURI_INTERNALS__.invoke;
  window.__themeWrites = [];
  window.__TAURI_INTERNALS__.invoke = async (command, args) => {
    if (command === 'get_color_theme_cmd')
      return JSON.parse(
        sessionStorage.getItem('theme-test-receipt') || '{"value":"dark","revision":1}',
      );
    if (command === 'set_color_theme_cmd') {
      const receipt = { value: args.value, revision: args.expectedRevision + 1 };
      sessionStorage.setItem('theme-test-receipt', JSON.stringify(receipt));
      window.__themeWrites.push(receipt);
      return receipt;
    }
    return base(command, args);
  };
}

test(
  'Tessera palettes match their source, switch and survive a frontend reload',
  { timeout: 45000 },
  async () => {
    const tokens = JSON.parse(
      await readFile(
        new URL('../src/shared/theme/config/tesseraTokens.json', import.meta.url),
        'utf8',
      ),
    );
    const server = await serveProduction();
    let browser;
    try {
      browser = await chromium.launch({ headless: true });
      const page = await browser.newPage({ viewport: { width: 1440, height: 1000 } });
      const errors = [];
      page.on('pageerror', (error) => errors.push(error.message));
      await page.addInitScript(installFixture);
      await page.addInitScript(installThemeFixture);
      await page.goto(server.url);
      await page.getByRole('button', { name: 'Settings', exact: true }).click();
      const choice = page.getByLabel('Color theme', { exact: true });
      await choice.waitFor();
      await choice.locator('option').first().waitFor({ state: 'attached' });
      assert.deepEqual(
        await choice.locator('option').evaluateAll((items) => items.map((item) => item.value)),
        ['light', 'dark'],
      );
      for (const theme of ['light', 'dark']) {
        await choice.selectOption(theme);
        await page.waitForFunction(
          (value) => document.documentElement.dataset.colorTheme === value,
          theme,
        );
        await page.waitForFunction(
          (value) => JSON.parse(sessionStorage.getItem('theme-test-receipt')).value === value,
          theme,
        );
        await page.evaluate(() => document.fonts.ready);
        const palette = await page.evaluate(() => {
          const style = getComputedStyle(document.documentElement);
          return Object.fromEntries(
            [
              'canvas',
              'raised',
              'panel',
              'thumbnail',
              'border',
              'borderStrong',
              'text',
              'textSecondary',
              'textMuted',
              'hover',
              'pressed',
              'primary',
              'primaryText',
              'accent',
            ].map((key) => [
              key,
              style
                .getPropertyValue(`--ts-color-${key}`)
                .trim()
                .toUpperCase()
                .replace(/^#([0-9A-F])([0-9A-F])([0-9A-F])$/, '#$1$1$2$2$3$3'),
            ]),
          );
        });
        assert.deepEqual(palette, tokens.themes[theme]);
        assert.match(
          await page.locator('body').evaluate((element) => getComputedStyle(element).fontFamily),
          /Cascadia Code/,
        );
        if (process.env.AURALIS_THEME_EVIDENCE)
          await page.screenshot({
            path: path.join(process.env.AURALIS_THEME_EVIDENCE, `tessera-${theme}.png`),
            fullPage: true,
          });
      }
      await choice.selectOption('light');
      await page.waitForFunction(
        () => JSON.parse(sessionStorage.getItem('theme-test-receipt')).value === 'light',
      );
      await page.reload();
      await page.waitForFunction(() => document.documentElement.dataset.colorTheme === 'light');
      assert.deepEqual(errors, []);
    } finally {
      await browser?.close();
      await server.close();
    }
  },
);
