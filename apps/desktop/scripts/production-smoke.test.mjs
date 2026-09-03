import assert from 'node:assert/strict';
import { test } from 'node:test';
import { chromium } from 'playwright';
import { serveProduction } from './production-server.mjs';
import { installFixture } from './production-fixture.mjs';

test(
  'production chunks load under CSP and keep global jobs across project navigation',
  { timeout: 45000 },
  async () => {
    const server = await serveProduction();
    let browser;
    try {
      browser = await chromium.launch({ headless: true });
      const page = await browser.newPage({ viewport: { width: 1280, height: 800 } });
      const errors = [];
      page.on('pageerror', (error) => errors.push(error.message));
      await page.addInitScript(installFixture);
      await page.goto(server.url);
      await page.getByRole('heading', { name: 'Projects', exact: true }).waitFor();
      const queue = page.getByRole('button', { name: /Очередь/ });
      await queue.click();
      await page.getByText('Background fixture job', { exact: true }).last().waitFor();
      await page.keyboard.press('Escape');
      await page.getByRole('button', { name: /^Open Review fixture/ }).click();
      await page.getByTestId('source-workspace').waitFor();
      await queue.click();
      await page.getByText('Background fixture job', { exact: true }).last().waitFor();
      await page.keyboard.press('Escape');
      await page.getByRole('button', { name: 'Settings', exact: true }).click();
      await page.getByRole('heading', { name: 'Settings', exact: true }).waitFor();
      await page.getByRole('button', { name: 'Back', exact: true }).click();
      await page.getByTestId('source-workspace').waitFor();
      assert.deepEqual(errors, []);
      assert.deepEqual(await page.evaluate(() => window.__smokeUnknownCommands), []);
      assert.deepEqual(await page.evaluate(() => window.__smokeViolations), []);
      assert.ok((await page.evaluate(() => window.__smokeCommands)).includes('list_jobs_cmd'));

      await page.evaluate(async () => {
        const script = document.createElement('script');
        script.textContent = 'window.__smokeInlineExecuted = true';
        document.body.append(script);
        try {
          await fetch('https://example.invalid/csp-probe');
        } catch {
          /* Expected CSP rejection. */
        }
      });
      await page.waitForFunction(() => window.__smokeViolations.includes('connect-src'));
      assert.equal(await page.evaluate(() => window.__smokeInlineExecuted), undefined);
      assert.ok((await page.evaluate(() => window.__smokeViolations)).includes('script-src-elem'));
    } finally {
      await browser?.close();
      await server.close();
    }
  },
);
