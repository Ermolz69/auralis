import assert from 'node:assert/strict';
import fsp from 'node:fs/promises';
import path from 'node:path';
import { chromium } from 'playwright';
import { installE2EFixture } from './e2e-fixture.mjs';
import { installUiTranslationFixture } from './ui-translation-fixture.mjs';
import { installUiSpeechFixture } from './ui-speech-fixture.mjs';
import { serveProduction } from './production-server.mjs';
import { uniqueRun } from '../../../tools/ui-e2e/core.mjs';
import { Report } from '../../../tools/ui-e2e/report.mjs';
import { story, storySteps } from '../../../tools/ui-e2e/story.mjs';
import { playback } from '../../../tools/ui-e2e/media.mjs';

const run = await uniqueRun('fixture-ui');
const report = new Report(run, 'fixture-ui', [
  'launch',
  ...storySteps.filter((id) => !['verify-artifacts', 'restart-playback'].includes(id)),
  'timing-conflict',
  'stale-preview',
  'assembly-cancel-resume',
  'reload-playback',
  'verification-error',
  'stale-approval',
  'fixture-integrity',
  'cleanup',
]);
let browser, server, page;
try {
  if (
    await report.step('launch', async () => {
      server = await serveProduction();
      browser = await chromium.launch({ headless: true });
      const context = await browser.newContext({ viewport: { width: 1280, height: 900 } });
      await context.tracing.start({ screenshots: true, snapshots: true, sources: false });
      page = await context.newPage();
      await page.addInitScript({
        content: `(${installE2EFixture.toString()})();(${installUiTranslationFixture.toString()})();(${installUiSpeechFixture.toString()})();`,
      });
      await page.goto(server.url);
      await page.getByLabel('YouTube URL', { exact: true }).waitFor();
    })
  ) {
    const complete = await story(
      page,
      report,
      {
        source_url: 'https://www.youtube.com/watch?v=HRaa8kb1EhE',
        allow_inference: true,
        edit_text: 'Здравствуйте, мир.',
        engines: ['voxcpm2', 'espeech'].map((engine) => ({
          engine_id: engine,
          profile_id: `native-plan-${engine}`,
          voice_id: 'fixture-speaker',
          reference_id: `fixture-${engine}`,
        })),
      },
      () => 60000,
      async () => ({ simulated: true }),
      async () => async () => ({ simulated: true, released: true }),
    );
    if (complete) {
      const assembly = page.getByRole('region', { name: 'Сборка итогового аудио', exact: true });
      await report.step(
        'timing-conflict',
        async () => {
          await assembly
            .getByLabel('Режим тайминга', { exact: true })
            .selectOption('subtitle_window');
          await assembly
            .getByLabel('Длительность отдельной аудиолинии, мс', { exact: true })
            .fill('1');
          await assembly
            .getByRole('button', { name: 'Измерить и показать тайминг', exact: true })
            .click();
          await assembly.getByRole('alert').waitFor();
          assert.equal(
            await assembly
              .getByRole('button', { name: 'Подготовить сборку', exact: true })
              .isEnabled(),
            false,
          );
        },
        page,
      );
      await report.step(
        'stale-preview',
        async () => {
          await assembly.getByLabel('Режим тайминга', { exact: true }).selectOption('continuous');
          await assembly
            .getByRole('button', { name: 'Измерить и показать тайминг', exact: true })
            .click();
          await assembly.getByRole('checkbox').check();
          await assembly
            .getByLabel('Пауза после fixture-utterance, мс', { exact: true })
            .fill('100');
          assert.equal(
            await assembly
              .getByRole('button', { name: 'Подготовить сборку', exact: true })
              .isEnabled(),
            false,
          );
          await assembly.getByLabel('Пауза после fixture-utterance, мс', { exact: true }).fill('0');
          await assembly
            .getByRole('button', { name: 'Измерить и показать тайминг', exact: true })
            .click();
          await assembly.getByRole('checkbox').check();
        },
        page,
      );
      await report.step(
        'assembly-cancel-resume',
        async () => {
          await assembly.getByRole('button', { name: 'Подготовить сборку', exact: true }).click();
          await assembly.getByRole('button', { name: 'Собрать WAV', exact: true }).click();
          await assembly.getByRole('button', { name: 'Отменить сборку', exact: true }).click();
          await assembly.getByRole('button', { name: 'Продолжить сборку', exact: true }).click();
          await assembly
            .getByRole('button', { name: 'Прослушать итоговый WAV', exact: true })
            .waitFor();
        },
        page,
      );
      await report.step(
        'reload-playback',
        async () => {
          await page.reload();
          await page.getByRole('button', { name: /^Open YouTube project/ }).click();
          await page.getByRole('button', { name: /Текст, субтитры и озвучка/ }).click();
          await page
            .getByLabel('Готовый исходный синтез', { exact: true })
            .selectOption('fixture-raw');
          await page
            .getByLabel('Сохранённые сборки', { exact: true })
            .selectOption('fixture-assembly');
          await page.getByRole('button', { name: 'Прослушать итоговый WAV', exact: true }).click();
          return playback(page, 'Итоговое аудио');
        },
        page,
      );
      await report.step(
        'verification-error',
        async () => {
          await page.reload();
          await page.getByRole('button', { name: /^Open YouTube project/ }).click();
          await page.getByRole('button', { name: /Текст, субтитры и озвучка/ }).click();
          await page
            .getByLabel('Готовый исходный синтез', { exact: true })
            .selectOption('fixture-raw');
          await page
            .getByLabel('Сохранённые сборки', { exact: true })
            .selectOption('fixture-assembly');
          await page.evaluate(() => {
            window.__ui.verifyFailure = true;
          });
          await page.getByRole('button', { name: 'Прослушать итоговый WAV', exact: true }).click();
          await page
            .getByRole('alert')
            .filter({ hasText: 'Fixture integrity verification failed' })
            .waitFor();
          assert.equal(await page.getByLabel('Итоговое аудио', { exact: true }).count(), 0);
        },
        page,
      );
      await report.step(
        'stale-approval',
        async () => {
          const review = page.getByRole('region', { name: 'Переводы проекта', exact: true });
          await review
            .getByRole('button', { name: 'Исправить перевод', exact: true })
            .first()
            .click();
          await review
            .getByLabel('Строка перевода 1', { exact: true })
            .fill('Новая версия без одобрения.');
          await review.getByRole('button', { name: 'Сохранить новую версию', exact: true }).click();
          await review.getByText('Решение ещё не принято', { exact: false }).waitFor();
          assert.equal(await review.getByText('Одобрено:', { exact: false }).count(), 0);
          const speech = page.getByRole('region', { name: 'Голоса синтеза речи', exact: true });
          await speech.getByRole('button', { name: 'Субтитры', exact: true }).click();
          await speech.getByRole('button', { name: 'Обновить источники', exact: true }).click();
          const input = speech.getByLabel('Источник субтитров', { exact: true });
          const option = await input
            .locator('option[value^="translation:"]')
            .first()
            .getAttribute('value');
          await input.selectOption(option);
          await speech
            .getByRole('button', { name: 'Использовать одобренный перевод', exact: true })
            .click();
          await speech
            .getByRole('alert')
            .filter({ hasText: 'Fixture translation approval is required' })
            .waitFor();
          assert.equal(
            await speech
              .getByRole('button', { name: 'Подготовить озвучку', exact: true })
              .isEnabled(),
            false,
          );
        },
        page,
      );
      await report.step(
        'fixture-integrity',
        async () => {
          const observed = await page.evaluate(() => ({
            calls: window.__ui.calls,
            unknown: window.__e2e.unknownCommands,
          }));
          assert.deepEqual(observed.unknown, []);
          assert.equal(
            observed.calls.filter((c) => c.command === 'start_speech_run_cmd').length,
            2,
          );
          assert.equal(
            observed.calls.filter((c) => c.command === 'resume_speech_run_cmd').length,
            2,
          );
          return {
            ui_only_actions: true,
            simulated_ipc: true,
            model_execution: false,
            commands: observed.calls.map((c) => c.command),
          };
        },
        page,
      );
    }
  }
} finally {
  await report.step('cleanup', async () => {
    if (page) {
      await fsp.writeFile(
        path.join(run.directory, 'dom.txt'),
        await page
          .locator('body')
          .innerText()
          .catch(() => ''),
      );
      await page.context().tracing.stop({ path: path.join(run.directory, 'trace.zip') });
    }
    await browser?.close();
    await server?.close();
  });
  await report.save();
  process.stdout.write(`UI report: ${run.id}\n`);
  process.exitCode = report.exitCode();
}
