import assert from 'node:assert/strict';
import { test } from 'node:test';
import path from 'node:path';
import { chromium } from 'playwright';
import { serveProduction } from './production-server.mjs';
import { installFixture } from './production-fixture.mjs';
import { installLibraryFixture } from './library-fixture.mjs';
import { installSpeechFixture } from './library-speech-fixture.mjs';

test(
  'menus, contextual tools, copies, settings and persistent workspaces',
  { timeout: 90000 },
  async () => {
    const server = await serveProduction();
    let browser;
    try {
      browser = await chromium.launch({ headless: true });
      const page = await browser.newPage({ viewport: { width: 1500, height: 1000 } });
      const errors = [];
      page.on('pageerror', (e) => errors.push(e.message));
      await page.addInitScript(installFixture);
      await page.addInitScript(installLibraryFixture);
      await page.addInitScript(installSpeechFixture);
      await page.goto(server.url);
      await page.getByRole('heading', { name: 'Все материалы', exact: true }).waitFor();
      const main = page.getByRole('menubar', { name: 'Главное меню' });
      assert.equal(await main.getByRole('menuitem').count(), 3);
      assert.equal(
        await page.getByRole('button', { name: 'Открыть инструмент', exact: true }).count(),
        0,
      );
      assert.equal(
        await page.getByRole('button', { name: 'Импорт файлов', exact: true }).count(),
        0,
      );
      const standalone = page.getByRole('button', { name: /Самостоятельный текст Текст/ });
      await standalone.dblclick();
      const input = page.getByRole('textbox', { name: 'Русский текст для озвучки' });
      await input.waitFor();
      assert.equal(await input.inputValue(), 'Расстояние 12 км.');
      await input.fill('Мой сохранённый черновик.');
      await page.waitForFunction(() =>
        JSON.parse(sessionStorage.getItem('library-state')).workspaces.some((w) =>
          w.tabs.some((t) => t.draft.includes('Мой сохранённый черновик.')),
        ),
      );
      const tools = main.getByRole('menuitem', { name: 'Инструменты', exact: true });
      await tools.click();
      const toolMenu = page.getByRole('menu', { name: 'Инструменты', exact: true });
      assert.equal(await toolMenu.getByRole('menuitem').count(), 6);
      assert.equal(
        await toolMenu
          .getByRole('menuitem', { name: /Источник и субтитры/ })
          .getAttribute('aria-disabled'),
        'true',
      );
      await toolMenu.getByRole('menuitem', { name: 'Синтез речи', exact: true }).click();
      assert.equal(await input.inputValue(), 'Мой сохранённый черновик.');
      await tools.click();
      await page
        .getByRole('menu')
        .getByRole('menuitem', { name: 'Синтез речи', exact: true })
        .click();
      assert.equal(await page.getByRole('tab').count(), 2);
      const settings = page.getByRole('button', { name: 'Настройки', exact: true });
      await settings.click();
      await page.getByRole('dialog', { name: 'Настройки', exact: true }).waitFor();
      await page.keyboard.press('Escape');
      assert.equal(await input.inputValue(), 'Мой сохранённый черновик.');
      assert.ok(await settings.evaluate((e) => e === document.activeElement));
      const logo = page.getByRole('button', { name: 'Auralis — библиотека', exact: true });
      await logo.click();
      await standalone.click();
      const other = page.getByRole('button', { name: /Общий рассказ Текст/ });
      await other.click({ button: 'right' });
      await page.getByRole('menuitem', { name: 'Свойства…', exact: true }).click();
      await page.getByRole('dialog').filter({ hasText: 'Общий рассказ' }).waitFor();
      await page.keyboard.press('Escape');
      await standalone.focus();
      await page.keyboard.press('Shift+F10');
      await page.getByRole('menu', { name: 'Действия' }).waitFor();
      await page.keyboard.press('Escape');
      assert.ok(await standalone.evaluate((e) => e === document.activeElement));
      const beta = page.getByRole('button', { name: 'Серия 02', exact: true });
      const drop = async () => {
        await standalone.and(page.locator('[aria-pressed="true"]')).dragTo(beta);
        await page.waitForFunction(() =>
          JSON.parse(sessionStorage.getItem('library-state')).links.some(
            (l) => l.projectId === window.__libraryIds.beta,
          ),
        );
      };
      await drop();
      await page.waitForFunction(
        () =>
          !document.querySelector('[role="status"]')?.textContent?.includes('Добавление в проект'),
      );
      const count = await page.evaluate(
        () => JSON.parse(sessionStorage.getItem('library-state')).assets.length,
      );
      await drop();
      assert.equal(
        await page.evaluate(
          () => JSON.parse(sessionStorage.getItem('library-state')).assets.length,
        ),
        count,
      );
      await beta.dblclick();
      await page.getByRole('complementary', { name: 'Материалы текущего проекта' }).waitFor();
      await main.getByRole('menuitem', { name: 'Файл', exact: true }).click();
      await page.getByRole('menuitem', { name: 'Новый текст…', exact: true }).click();
      let dialog = page.getByRole('dialog');
      await dialog.getByText('Серия 02', { exact: false }).waitFor();
      await dialog.getByLabel('Название', { exact: true }).fill('Текст главы');
      await dialog.getByLabel('Текст', { exact: true }).fill('Первая глава.');
      await dialog.getByRole('button', { name: 'Создать', exact: true }).click();
      await input.waitFor();
      assert.equal(await input.inputValue(), 'Первая глава.');
      const resources = page.getByRole('complementary', { name: 'Материалы текущего проекта' });
      await resources.getByRole('button', { name: /Самостоятельный текст/ }).click();
      assert.equal(await input.inputValue(), 'Первая глава.');
      await tools.click();
      await page.getByRole('menuitem', { name: 'Текст и подготовка', exact: true }).click();
      await page.waitForFunction(() =>
        [...document.querySelectorAll('textarea')].some(
          (e) => e.getClientRects().length && e.value === 'Расстояние 12 км.',
        ),
      );
      await tools.focus();
      await page.keyboard.press('ArrowDown');
      await page.keyboard.press('ArrowDown');
      await page.keyboard.press('Escape');
      assert.ok(await tools.evaluate((e) => e === document.activeElement));
      for (const theme of ['dark', 'light']) {
        await page.evaluate(
          (theme) => document.documentElement.setAttribute('data-color-theme', theme),
          theme,
        );
        for (const width of [1500, 800]) {
          await page.setViewportSize({ width, height: 900 });
          assert.equal(
            await page.evaluate(() => document.documentElement.scrollWidth > innerWidth),
            false,
          );
          if (process.env.AURALIS_LIBRARY_EVIDENCE)
            await page.screenshot({
              path: path.join(
                process.env.AURALIS_LIBRARY_EVIDENCE,
                `media-menu-${theme}-${width}.png`,
              ),
              fullPage: true,
            });
        }
      }
      assert.equal(
        (await page.evaluate(() => window.__puzaCalls)).filter(
          (c) => c.command === 'start_speech_run_cmd',
        ).length,
        0,
      );
      const processes = page.getByRole('region', { name: 'Текущие процессы' });
      await settings.click();
      await page.evaluate(() => {
        for (const id of window.__smokeListenerGroups.get('job-event'))
          window.__smokeCallbacks.get(id)({
            event: 'job-event',
            id: 1,
            payload: {
              kind: 'progressed',
              job: {
                ...window.__smokeJob,
                revision: 2,
                progress: { ...window.__smokeJob.progress, percent: 64 },
              },
            },
          });
      });
      await page.keyboard.press('Escape');
      await processes.getByText('64%', { exact: true }).waitFor();
      await logo.click();
      await page.evaluate(() => {
        window.__libraryRuntimeMissing = true;
      });
      await main.getByRole('menuitem', { name: 'Файл', exact: true }).click();
      await page.getByRole('menuitem', { name: 'Новый проект…', exact: true }).click();
      dialog = page.getByRole('dialog');
      await dialog.getByLabel('Название', { exact: true }).fill('Недоступный проект');
      await dialog.getByRole('button', { name: 'Создать', exact: true }).click();
      await dialog
        .getByRole('alert')
        .filter({ hasText: 'CONTRACT runtime is unavailable' })
        .waitFor();
      assert.deepEqual(errors, []);
      assert.deepEqual(await page.evaluate(() => window.__smokeViolations), []);
    } finally {
      await browser?.close();
      await server.close();
    }
  },
);
