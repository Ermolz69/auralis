import assert from 'node:assert/strict';
import { test } from 'node:test';
import { chromium } from 'playwright';
import { serveProduction } from './production-server.mjs';
import { installFixture } from './production-fixture.mjs';

function installSpeechFixture() {
  const base = window.__TAURI_INTERNALS__.invoke;
  const digest = 'a'.repeat(64);
  const source = (text) => ({
    kind: 'text',
    source_id: null,
    revision: digest,
    sha256: digest,
    text,
    cues: [],
  });
  const voices = ['ciri', 'geralt'].flatMap((voice) =>
    ['voxcpm2', 'espeech'].map((engine) => ({
      voice_id: `${voice}_${engine}`,
      display_name: voice === 'ciri' ? 'Ciri' : 'Geralt',
      description: '',
      manifest_sha256: digest,
      references: [
        {
          reference_id: 'neutral-v2',
          engine_id: engine,
          emotion_id: 'neutral',
          transcript: 'Нейтральный образец.',
          audio_sha256: digest,
          qualification_status: 'qualified',
        },
      ],
    })),
  );
  let saved = null;
  let planned = null;
  window.__puzaCalls = [];
  window.__TAURI_INTERNALS__.invoke = async (command, args) => {
    window.__puzaCalls.push({ command, args });
    if (command === 'get_speech_catalog_cmd')
      return {
        voices,
        profiles: ['voxcpm2', 'espeech'].map((engine) => ({
          profile_id: `native-plan-${engine}`,
          engine_id: engine,
          profile_sha256: digest,
        })),
      };
    if (command === 'get_speech_voice_selection_cmd') return saved;
    if (command === 'save_speech_voice_selection_cmd')
      return (saved = {
        role_id: 'narrator',
        display_name: 'Рассказчик',
        voice_id: args.voiceId,
        reference_id: args.referenceId,
        engine_id: args.profileId.endsWith('espeech') ? 'espeech' : 'voxcpm2',
      });
    if (
      [
        'list_project_artifacts_cmd',
        'list_project_translations_cmd',
        'list_ready_translation_edits_cmd',
      ].includes(command)
    )
      return [];
    if (command === 'list_speech_runs_cmd') return planned ? [planned] : [];
    if (command === 'get_speech_run_cmd') return planned;
    if (command === 'prepare_speech_text_cmd')
      return {
        version: '1.0.0',
        original: source(args.source.text),
        prepared: source('Расстояние двенадцать км.'),
        changed_targets: 1,
        unresolved_targets: 0,
      };
    if (command === 'review_prepared_speech_text_cmd')
      return {
        decision_id: 'review-fixture',
        project_id: args.projectId,
        source: args.preparation.prepared,
        original_source: args.preparation.original,
      };
    if (command === 'prepare_speech_plan_cmd')
      return (planned = {
        run_id: 'run-fixture',
        profile_id: args.request.profile_id,
        utterance_count: 1,
        chunk_count: 1,
        job: null,
        result: null,
      });
    return base(command, args);
  };
}

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
      await page.getByRole('button', { name: /^Open Review fixture/ }).click();
      await page.getByRole('button', { name: /Текст, субтитры и озвучка/ }).click();
      const panel = page.getByRole('region', { name: 'Голоса синтеза речи' });
      await panel.getByText('Проект проверен · библиотека голосов подключена').waitFor();
      assert.equal(
        await panel.getByRole('button', { name: 'Подготовить озвучку', exact: true }).isEnabled(),
        false,
      );
      await panel.getByLabel('Русский текст для озвучки').fill('Расстояние 12 км.');
      await panel.getByRole('button', { name: 'Подготовить текст', exact: true }).click();
      await panel.getByText('Предлагаемый вариант', { exact: true }).waitFor();
      await panel
        .getByRole('button', { name: 'Использовать подготовленный текст', exact: true })
        .click();
      await panel.getByRole('button', { name: 'Текст проверен', exact: true }).click();
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
