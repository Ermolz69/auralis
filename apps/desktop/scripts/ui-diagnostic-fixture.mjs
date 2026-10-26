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
import { story, workflowSteps } from '../../../tools/ui-e2e/story.mjs';
import { reopenPlayback, workflowRecords } from '../../../tools/ui-e2e/reopen.mjs';
import { frontendIdentity } from '../../../tools/ui-e2e/frontend.mjs';

const spec = {
  source_url: 'https://www.youtube.com/watch?v=HRaa8kb1EhE',
  case_scope: 'diagnostic',
  expected_cue_count: 1,
  allow_inference: true,
  allow_unverified_neutral: true,
  diagnostic_text: 'Короткая проверка сохранённого текста.',
  engines: ['voxcpm2', 'espeech'].map((engine_id) => ({
    engine_id,
    profile_id: `native-plan-${engine_id}`,
    voice_id: 'fixture-speaker',
    reference_id: `fixture-${engine_id}`,
  })),
};
const run = await uniqueRun('fixture-diagnostic');
const report = new Report(run, 'fixture-diagnostic', [
  'launch',
  ...workflowSteps(spec).filter((id) => !['verify-artifacts', 'restart-playback'].includes(id)),
  'reload-both-engines',
  'fixture-integrity',
  'javascript-errors',
  'cleanup',
]);
report.data.coverage = {
  simulated_ipc: true,
  model_execution: false,
  source_fixture_cues: 1,
  tts_utterances: 1,
  translation: 'not_run',
  mp4: 'not_run',
};
let context, server, page;
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
        content: `(${installE2EFixture.toString()})();(${installUiTranslationFixture.toString()})();(${installUiSpeechFixture.toString()})();window.__ui.completeSpeechOnStart=true;`,
      });
      await page.goto(server.url);
      await page.getByLabel('YouTube URL', { exact: true }).waitFor();
      await page.screenshot({ path: path.join(run.directory, 'launch.png') });
    })
  ) {
    const complete = await story(
      page,
      report,
      spec,
      () => 60000,
      async () => ({ simulated: true }),
      async () => async () => ({ simulated: true, released: true }),
    );
    if (complete) {
      await report.step(
        'reload-both-engines',
        async () => {
          const records = workflowRecords(report, spec);
          await page.reload();
          return reopenPlayback(page, records, () => 60000);
        },
        page,
      );
      await report.step(
        'fixture-integrity',
        async () => {
          const { calls, unknown } = await page.evaluate(() => ({
            calls: window.__ui.calls,
            unknown: window.__e2e.unknownCommands,
          }));
          assert.deepEqual(unknown, []);
          assert.equal(calls.filter((c) => c.command === 'start_speech_run_cmd').length, 2);
          assert.equal(
            new Set(
              calls.filter((c) => c.command === 'start_speech_run_cmd').map((c) => c.args.runId),
            ).size,
            2,
          );
          assert.equal(calls.filter((c) => c.command === 'resume_speech_run_cmd').length, 0);
          assert.equal(calls.filter((c) => c.command === 'start_translation_run_cmd').length, 0);
          for (const call of calls.filter((c) => c.command === 'review_speech_text_cmd'))
            assert.equal(call.args.source.text, spec.diagnostic_text);
          return {
            single_start_per_engine: true,
            models_loaded: false,
            translation: 'not_run',
            mp4: 'not_run',
          };
        },
        page,
      );
    }
    await report.step(
      'javascript-errors',
      () => {
        assert.deepEqual(errors, []);
        return { errors: 0 };
      },
      page,
    );
  }
} catch (error) {
  report.data.steps.push({ id: 'runner-error', status: 'failed', error: error.message });
} finally {
  await report.step('cleanup', async () => {
    try {
      if (page)
        await fsp.writeFile(
          path.join(run.directory, 'dom.txt'),
          await page.locator('body').innerText(),
        );
      if (context) await context.tracing.stop({ path: path.join(run.directory, 'trace.zip') });
    } finally {
      await context?.close();
      await server?.close();
    }
  });
  await report.save();
  process.stdout.write(`UI report: ${run.id}\n`);
  process.exitCode = report.exitCode();
}
