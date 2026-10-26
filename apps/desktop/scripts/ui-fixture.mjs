import assert from 'node:assert/strict';
import fsp from 'node:fs/promises';
import path from 'node:path';
import { chromium } from 'playwright';
import { installE2EFixture } from './e2e-fixture.mjs';
import { installUiTranslationFixture } from './ui-translation-fixture.mjs';
import { installUiSpeechFixture } from './ui-speech-fixture.mjs';
import { installUiVideoFixture } from './ui-video-fixture.mjs';
import { serveProduction } from './production-server.mjs';
import { uniqueRun } from '../../../tools/ui-e2e/core.mjs';
import { Report } from '../../../tools/ui-e2e/report.mjs';
import { story, workflowSteps } from '../../../tools/ui-e2e/story.mjs';
import { playback } from '../../../tools/ui-e2e/media.mjs';
import { reopenPlayback, workflowRecords } from '../../../tools/ui-e2e/reopen.mjs';
import { jobState } from '../../../tools/ui-e2e/observe.mjs';
import { frontendIdentity } from '../../../tools/ui-e2e/frontend.mjs';

const run = await uniqueRun('fixture-ui');
const report = new Report(run, 'fixture-ui', [
  'launch',
  ...workflowSteps({ export_video: true, exercise_cancellation: true }).filter(
    (id) => !['verify-artifacts', 'restart-playback'].includes(id),
  ),
  'reload-both-engines',
  'timing-conflict',
  'video-verification-error',
  'stale-preview',
  'assembly-error-resume',
  'assembly-cancel-resume',
  'reload-playback',
  'verification-error',
  'stale-approval',
  'fixture-integrity',
  'javascript-errors',
  'cleanup',
]);
let context, server, page;
let savedAssembly;
const errors = [];
try {
  if (
    await report.step('launch', async () => {
      report.data.versions = await frontendIdentity();
      server = await serveProduction();
      context = await chromium.launchPersistentContext(path.join(run.directory, 'profile'), {
        headless: true,
        viewport: { width: 1280, height: 900 },
      });
      await context.tracing.start({ screenshots: true, snapshots: true, sources: false });
      page = await context.newPage();
      page.on('pageerror', (error) => errors.push(error.message));
      await page.addInitScript({
        content: `(${installE2EFixture.toString()})();(${installUiTranslationFixture.toString()})();(${installUiSpeechFixture.toString()})();(${installUiVideoFixture.toString()})();`,
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
        allow_unverified_neutral: true,
        install_translation: true,
        export_video: true,
        exercise_cancellation: true,
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
      await report.step(
        'reload-both-engines',
        async () => {
          const records = workflowRecords(report, {
            engines: ['voxcpm2', 'espeech'].map((engine_id) => ({ engine_id })),
            export_video: true,
          });
          await page.reload();
          return reopenPlayback(page, records, () => 60000);
        },
        page,
      );
      const assembly = page.getByRole('region', { name: 'Сборка итогового аудио', exact: true });
      await report.step(
        'video-verification-error',
        async () => {
          await page.evaluate(() => {
            window.__ui.verifyFailure = true;
          });
          await assembly
            .getByRole('button', { name: 'Воспроизвести русское видео', exact: true })
            .click();
          await assembly
            .getByRole('alert')
            .filter({ hasText: 'Fixture integrity verification failed' })
            .waitFor();
          assert.equal(
            await page.getByLabel('Видео с русской озвучкой', { exact: true }).count(),
            0,
          );
          await page.evaluate(() => {
            window.__ui.verifyFailure = false;
          });
        },
        page,
      );
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
        'assembly-error-resume',
        async () => {
          await assembly.getByRole('button', { name: 'Подготовить сборку', exact: true }).click();
          await page.evaluate(() => {
            window.__ui.failNextAssembly = true;
          });
          await assembly.getByRole('button', { name: 'Собрать WAV', exact: true }).click();
          await assembly
            .getByRole('alert')
            .filter({ hasText: 'Fixture assembly execution failed' })
            .waitFor();
          await assert.rejects(
            jobState(assembly, 'Состояние сборки', ['отменено'], 1000),
            /Fixture assembly execution failed/,
          );
          assert.equal(
            await assembly
              .getByRole('button', { name: 'Прослушать итоговый WAV', exact: true })
              .count(),
            0,
          );
          const previous = {
            job_id: (
              await assembly
                .getByRole('status', { name: 'Состояние сборки', exact: true })
                .innerText()
            )
              .split(' · ')[0]
              .replace(/^Задание /, ''),
          };
          await assembly.getByRole('button', { name: 'Продолжить сборку', exact: true }).click();
          return jobState(assembly, 'Состояние сборки', ['готово'], 10000, previous);
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
          savedAssembly = await assembly
            .getByLabel('Сохранённые сборки', { exact: true })
            .inputValue();
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
            .getByLabel('Сохранённые запуски', { exact: true })
            .selectOption('fixture-raw-espeech');
          await page
            .getByText(
              'Технический прогон с неподтверждённым образцом. Качество озвучки не принято.',
              { exact: true },
            )
            .waitFor();
          await page
            .getByLabel('Готовый исходный синтез', { exact: true })
            .selectOption('fixture-raw-espeech');
          await page.getByLabel('Сохранённые сборки', { exact: true }).selectOption(savedAssembly);
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
            .selectOption('fixture-raw-espeech');
          await page.getByLabel('Сохранённые сборки', { exact: true }).selectOption(savedAssembly);
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
          await speech
            .getByRole('checkbox', {
              name: 'Разрешаю неподтверждённый нейтральный образец для технического прогона',
              exact: true,
            })
            .check();
          await speech.getByLabel('Движок', { exact: true }).selectOption('espeech');
          await speech.getByLabel('Голос', { exact: true }).selectOption('fixture-speaker');
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
          const imports = observed.calls.filter(
            (c) => c.command === 'start_project_mock_pipeline_cmd',
          );
          assert.equal(imports.length, 1);
          assert.equal(imports[0].args.subtitleTrackId, 'zh-srt');
          assert.equal(imports[0].args.subtitleLanguage, 'zh');
          assert.equal(imports[0].args.subtitleAutoGenerated, false);
          assert.equal(
            observed.calls.filter((c) => c.command === 'start_speech_run_cmd').length,
            2,
          );
          assert.equal(
            observed.calls.filter((c) => c.command === 'resume_speech_run_cmd').length,
            2,
          );
          const starts = observed.calls.filter((c) => c.command === 'start_speech_run_cmd');
          assert.equal(
            new Set(starts.map((c) => c.args.runId)).size,
            2,
            'Duplicate start reused a run',
          );
          const state = await page.evaluate(() => ({
            speech: Object.values(window.__ui.speeches),
            assemblies: Object.values(window.__ui.assemblies),
          }));
          assert.equal(new Set(state.speech.map((r) => r.job.id)).size, 2);
          assert.equal(new Set(state.speech.map((r) => r.result.attempt_id)).size, 2);
          assert.equal(state.assemblies.length, 6);
          assert.equal(new Set(state.assemblies.map((r) => r.job.id)).size, 6);

          assert.equal(
            observed.calls.filter((c) => c.command === 'install_translation_package_cmd').length,
            1,
          );
          assert.equal(
            observed.calls.filter((c) => c.command === 'select_translation_package_cmd').length,
            1,
          );
          assert.equal(
            observed.calls.filter(
              (c) =>
                c.command === 'prepare_speech_plan_cmd' &&
                c.args.request.allow_unverified_neutral === true,
            ).length,
            2,
          );
          return {
            ui_only_actions: true,
            simulated_ipc: true,
            model_execution: false,
            video_fixture_codec: 'synthetic WebM/VP8; MP4 encoding is not exercised by fixture IPC',
            commands: observed.calls.map((c) => c.command),
          };
        },
        page,
      );
    }
  }
  if (page)
    await report.step(
      'javascript-errors',
      () => {
        assert.deepEqual(errors, []);
        return { errors: 0 };
      },
      page,
    );
} catch (error) {
  report.data.steps.push({ id: 'runner-error', status: 'failed', error: error.message });
} finally {
  await report.step('cleanup', async () => {
    try {
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
    } finally {
      try {
        await context?.close();
      } finally {
        await server?.close();
      }
    }
  });
  await report.save();
  process.stdout.write(`UI report: ${run.id}\n`);
  process.exitCode = report.exitCode();
}
