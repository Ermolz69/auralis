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
      await page.addInitScript(installFixture, { assets: [{
        id: 'review-project', ownerProjectId: 'review-project', name: 'Source fixture',
        kind: 'video', artifactId: null, text: null, format: null, parentId: null, tags: [], openedAt: null,
      }] });
      await page.addInitScript(() => {
        window.__smokeJob.status = 'pending';
        window.__smokeJob.progress.percent = 0;
      });
      await page.goto(server.url);
      await page.getByRole('heading', { name: 'Все материалы', exact: true }).waitFor();
      const titlebar = page.getByRole('banner', { name: 'Панель окна' });
      assert.equal((await titlebar.boundingBox()).height, 36);
      assert.equal(
        await titlebar.evaluate((element) => element.hasAttribute('data-tauri-drag-region')),
        true,
      );
      assert.equal(
        await titlebar
          .getByRole('button')
          .evaluateAll((elements) =>
            elements.every((element) => !element.hasAttribute('data-tauri-drag-region')),
          ),
        true,
      );
      await titlebar.getByRole('button', { name: 'Развернуть окно', exact: true }).click();
      await titlebar.getByRole('button', { name: 'Восстановить окно', exact: true }).waitFor();
      await titlebar.getByRole('button', { name: 'Восстановить окно', exact: true }).click();
      await titlebar.getByRole('button', { name: 'Развернуть окно', exact: true }).waitFor();
      await page.evaluate(() => {
        window.__smokeWindowFailure = true;
      });
      await titlebar.getByRole('button', { name: 'Свернуть окно', exact: true }).click();
      await page.getByRole('alert').filter({ hasText: 'Не удалось управлять окном' }).waitFor();
      await page.evaluate(() => {
        window.__smokeWindowFailure = false;
      });
      await titlebar.getByRole('button', { name: 'Свернуть окно', exact: true }).click();
      await page.waitForFunction(() => window.__smokeWindow.minimized);
      assert.equal(
        await page.getByRole('alert').filter({ hasText: 'Не удалось управлять окном' }).count(),
        0,
      );
      const queue = page.getByRole('region', { name: 'Текущие процессы' });
      await queue.getByText('Background fixture job', { exact: true }).waitFor();
      await queue.getByText('В очереди', { exact: true }).waitFor();
      assert.equal(await page.getByRole('button', { name: /^Библиотека$|Очередь/ }).count(), 0);
      assert.equal(
        await page.getByRole('combobox', { name: 'Открытые рабочие пространства' }).count(),
        0,
      );
      await page.getByRole('button', { name: 'Review fixture', exact: true }).dblclick();
      await page.getByRole('complementary', { name: 'Материалы текущего проекта' }).getByRole('button', { name: /Source fixture/ }).click();
      await page.getByRole('menubar', { name: 'Главное меню' }).getByRole('menuitem', { name: 'Инструменты', exact: true }).click();
      await page.getByRole('menuitem', { name: 'Источник и субтитры', exact: true }).click();
      await page.getByTestId('source-workspace').waitFor();
      await titlebar.getByRole('button', { name: 'Закрыть окно', exact: true }).click();
      await page.waitForFunction(() => window.__smokeWindow.closed);
      await queue.getByText('Background fixture job', { exact: true }).waitFor();
      const settings = page.getByRole('button', { name: 'Настройки', exact: true });
      assert.equal(await settings.locator('svg').count(), 1);
      assert.equal((await settings.textContent()).trim(), '');
      await settings.click();
      await page.getByRole('heading', { name: 'Настройки', exact: true }).waitFor();
      await queue.getByText('Background fixture job', { exact: true }).waitFor();
      await page.getByRole('button', { name: 'Закрыть настройки', exact: true }).click();
      await page.getByRole('button', { name: 'Auralis — библиотека', exact: true }).click();
      const project = page
        .getByRole('navigation', { name: 'Библиотека и проекты' })
        .getByRole('button', { name: 'Review fixture', exact: true });
      await project.focus();
      await page.keyboard.press('Enter');
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
