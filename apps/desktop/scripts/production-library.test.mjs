import assert from 'node:assert/strict';
import { test } from 'node:test';
import path from 'node:path';
import { chromium } from 'playwright';
import { serveProduction } from './production-server.mjs';
import { installFixture } from './production-fixture.mjs';
import { installLibraryFixture } from './library-fixture.mjs';
import { installSpeechFixture } from './library-speech-fixture.mjs';

test(
  'library and separate project/standalone workspaces preserve drafts, membership and versions',
  { timeout: 60000 },
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
      assert.equal(await page.getByRole('tablist').count(), 0);
      const standalone = page.getByRole('button', { name: /Самостоятельный текст Текст/ });
      await standalone.dblclick();
      await page.getByText('Самостоятельный материал', { exact: true }).waitFor();
      const input = page.getByRole('textbox', { name: 'Русский текст для озвучки' });
      await input.waitFor();
      assert.equal(await input.inputValue(), 'Расстояние 12 км.');
      await input.fill('Мой сохранённый черновик.');
      await page.waitForFunction(() =>
        JSON.parse(sessionStorage.getItem('library-state')).workspaces.some((w) =>
          w.tabs.some((t) => t.draft.includes('Мой сохранённый черновик.')),
        ),
      );
      await page.getByRole('button', { name: 'Синтез речи', exact: true }).click();
      assert.equal(await input.inputValue(), 'Мой сохранённый черновик.');
      await page.getByRole('button', { name: 'Библиотека', exact: true }).click();
      await page.getByRole('heading', { name: 'Все материалы', exact: true }).waitFor();
      assert.equal(await page.getByRole('tablist').count(), 0);

      await page.getByRole('button', { name: 'Серия 01', exact: true }).click();
      await page.getByRole('button', { name: '+ Папка', exact: true }).click();
      let dialog = page.getByRole('dialog');
      await dialog.getByLabel('Название', { exact: true }).fill('Главы');
      await dialog.getByRole('button', { name: 'Создать', exact: true }).click();
      await page.getByRole('button', { name: 'Главы', exact: true }).click();
      await page.getByRole('button', { name: 'Новый текст', exact: true }).click();
      dialog = page.getByRole('dialog');
      await dialog.getByLabel('Название', { exact: true }).fill('Текст главы');
      await dialog.getByLabel('Текст', { exact: true }).fill('Первая глава.');
      await dialog.getByRole('button', { name: 'Создать', exact: true }).click();
      await page.getByText('Проект', { exact: true }).waitFor();
      await input.waitFor();
      assert.equal(await input.inputValue(), 'Первая глава.');
      await page.getByRole('button', { name: 'Синтез речи', exact: true }).click();
      assert.equal(await input.inputValue(), 'Первая глава.');
      await page
        .getByRole('tab', { name: 'Текст и подготовка · Текст главы', exact: true })
        .click();
      await input.fill('Новая версия первой главы.');
      await page.getByRole('button', { name: 'Сохранить текст новой версией' }).click();
      await page.waitForFunction(() =>
        JSON.parse(sessionStorage.getItem('library-state')).assets.some(
          (a) => a.parentId && a.text === 'Новая версия первой главы.',
        ),
      );
      await page.getByRole('button', { name: 'Библиотека', exact: true }).click();
      await page.getByRole('button', { name: 'Все материалы', exact: true }).click();
      await standalone.click();
      const inspector = page.getByRole('complementary', { name: 'Выбранный материал' });
      await inspector
        .getByLabel('Проект', { exact: true })
        .selectOption(await page.evaluate(() => window.__libraryIds.beta));
      await inspector.getByRole('button', { name: 'Добавить / переместить в проект' }).click();
      await page.evaluate(() => {
        window.__libraryConflict = true;
      });
      await inspector.getByLabel('Теги через запятую').fill('общий, принятый');
      await inspector.getByRole('button', { name: 'Сохранить свойства' }).click();
      await page.waitForFunction(() =>
        JSON.parse(sessionStorage.getItem('library-state')).assets.some(
          (a) => a.id === window.__libraryIds.asset && a.tags.includes('принятый'),
        ),
      );
      await page.getByRole('button', { name: 'Серия 02', exact: true }).click();
      await standalone.click();
      await inspector.getByRole('button', { name: 'Независимая копия', exact: true }).click();
      await page.waitForFunction(() =>
        JSON.parse(sessionStorage.getItem('library-state')).assets.some((a) =>
          a.name.includes('Самостоятельный текст — копия'),
        ),
      );
      await page.waitForFunction(
        () =>
          !JSON.parse(sessionStorage.getItem('library-state')).links.some(
            (l) =>
              l.assetId === window.__libraryIds.asset && l.projectId === window.__libraryIds.beta,
          ),
      );
      await page.getByRole('button', { name: 'Все материалы', exact: true }).click();
      if (process.env.AURALIS_LIBRARY_EVIDENCE)
        await page.screenshot({
          path: path.join(process.env.AURALIS_LIBRARY_EVIDENCE, 'library-wide.png'),
          fullPage: true,
        });
      await page.reload();
      await page.getByTestId('library-browser').waitFor();
      await page
        .getByRole('combobox', { name: 'Открытые рабочие пространства' })
        .selectOption(`asset:${await page.evaluate(() => window.__libraryIds.asset)}`);
      await input.waitFor();
      assert.equal(await input.inputValue(), 'Мой сохранённый черновик.');
      await page.getByRole('button', { name: 'Очередь', exact: false }).click();
      await page.getByText('Background fixture job', { exact: true }).last().waitFor();
      await page.keyboard.press('Escape');
      if (process.env.AURALIS_LIBRARY_EVIDENCE)
        await page.screenshot({
          path: path.join(process.env.AURALIS_LIBRARY_EVIDENCE, 'workspace-wide.png'),
          fullPage: true,
        });
      await page.setViewportSize({ width: 800, height: 800 });
      if (process.env.AURALIS_LIBRARY_EVIDENCE)
        await page.screenshot({
          path: path.join(process.env.AURALIS_LIBRARY_EVIDENCE, 'workspace-compact.png'),
          fullPage: true,
        });
      assert.deepEqual(errors, []);
      assert.deepEqual(await page.evaluate(() => window.__smokeViolations), []);
      assert.equal(
        (await page.evaluate(() => window.__puzaCalls)).filter(
          (c) => c.command === 'start_speech_run_cmd',
        ).length,
        0,
      );
      await page.getByRole('button', { name: 'Библиотека', exact: true }).click();
      await page.evaluate(() => {
        window.__libraryRuntimeMissing = true;
      });
      await page.getByRole('button', { name: 'Создать проект', exact: true }).click();
      dialog = page.getByRole('dialog');
      await dialog.getByLabel('Название', { exact: true }).fill('Заблокированный проект');
      await dialog.getByRole('button', { name: 'Создать', exact: true }).click();
      await dialog
        .getByRole('alert')
        .filter({ hasText: 'PUZA CONTRACT runtime is unavailable' })
        .waitFor();
      assert.equal(
        await page.evaluate(() =>
          JSON.parse(sessionStorage.getItem('library-projects')).some(
            (p) => p.title === 'Заблокированный проект',
          ),
        ),
        false,
      );
    } finally {
      await browser?.close();
      await server.close();
    }
  },
);
