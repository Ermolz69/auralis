import assert from 'node:assert/strict';
import { test } from 'node:test';
import { chromium } from 'playwright';
import { serveProduction } from './production-server.mjs';
import { installFixture } from './production-fixture.mjs';
import { installSpeechFixture } from './library-speech-fixture.mjs';


test(
  'production speech panel previews, confirms and prepares explicit user selections',
  { timeout: 45000 },
  async () => {
    const server = await serveProduction();
    let browser;
    try {
      browser = await chromium.launch({ headless: true });
      const page = await browser.newPage({ viewport: { width: 1280, height: 1500 } });
      const errors = [];
      page.on('pageerror', (error) => errors.push(error.message));
      await page.addInitScript(installFixture);
      await page.addInitScript(installSpeechFixture);
      await page.goto(server.url);
      await page.getByRole('button', { name: 'Review fixture', exact: true }).dblclick();
      await page.getByRole('button', { name: 'Текст и подготовка', exact: true }).click();
      const panel = page.getByRole('region', { name: 'Голоса синтеза речи' });
      await panel.getByText('Проект проверен · библиотека голосов подключена').waitFor();
      await panel.getByLabel('Русский текст для озвучки').fill('Расстояние 12 км.');
      await panel.getByRole('button', { name: 'Подготовить текст', exact: true }).click();
      await panel.getByText('Предлагаемый вариант', { exact: true }).waitFor();
      await panel
        .getByRole('button', { name: 'Использовать подготовленный текст', exact: true })
        .click();
      await panel.getByRole('button', { name: 'Текст проверен', exact: true }).click();
      await page.getByRole('button', { name: 'Синтез речи', exact: true }).click();
      await panel.getByLabel('Движок', { exact: true }).selectOption('espeech');
      await panel.getByLabel('Голос', { exact: true }).selectOption('geralt_espeech');
      await panel.getByRole('button', { name: 'Сохранить голос проекта', exact: true }).click();
      await panel.getByText('Голос проекта сохранён и проверен.').waitFor();
      await panel.getByRole('button', { name: 'Подготовить озвучку', exact: true }).click();
      await panel.getByRole('button', { name: 'Озвучить', exact: true }).waitFor();
      assert.deepEqual(errors, []);
      const calls = await page.evaluate(() => window.__puzaCalls);
      assert.ok(
        calls.some(
          ({ command, args }) =>
            command === 'review_prepared_speech_text_cmd' && args.confirmed === true,
        ),
      );
      const plan = calls.find(({ command }) => command === 'prepare_speech_plan_cmd').args.request;
      assert.equal(plan.voice_id, 'geralt_espeech');
      assert.equal(plan.profile_id, 'native-plan-espeech');
      if (process.env.AURALIS_PUZA_SCREENSHOT)
        await panel.screenshot({ path: process.env.AURALIS_PUZA_SCREENSHOT });
    } finally {
      await browser?.close();
      await server.close();
    }
  },
);
