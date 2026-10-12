import { createProject, listProjects, renameProject } from '@/entities/project';
import { invoke } from '@/shared/api/tauri';

const POLL_MS = 250;
const TIMEOUT_MS = 900_000;
const longText =
  'Мы сохраняем проверенный текст и читаем его спокойно. Голос остаётся нейтральным, а каждое слово должно звучать ясно. '.repeat(
    9,
  );

export async function runSpeechScenario(runId: string, showProject: (id: string) => void) {
  let project = (await listProjects()).find((p) => p.title.startsWith('native-e2e-speech-'));
  project ??= await createProject(`native-e2e-speech-running:${runId}`);
  const projectId = project.id;
  try {
    showProject(projectId);
    await waitFor(() =>
      document.querySelector<HTMLSelectElement>('select[aria-label="Движок и профиль"]'),
    );
    if (
      [`native-e2e-speech-complete:${runId}`, `native-e2e-speech-reopened:${runId}`].includes(
        project.title,
      )
    ) {
      const ready = (await invoke('list_speech_runs_cmd', { projectId })).filter(
        (r) => r.result?.ready,
      );
      if (ready.length !== 10) throw new Error('Restart lost a ready speech result');
      await select('Сохранённые запуски', ready[0].run_id);
      await playback();
      await title(projectId, `native-e2e-speech-reopened:${runId}`);
      return;
    }
    if (project.title === `native-e2e-speech-cancelled:${runId}`) {
      const cancelled = (await invoke('list_speech_runs_cmd', { projectId })).find(
        (r) => r.job?.status === 'cancelled',
      );
      if (!cancelled) throw new Error('Restart lost the cancelled speech run');
      await select('Сохранённые запуски', cancelled.run_id);
      await click('Продолжить');
      await ready(projectId, cancelled.run_id);
      await playback();
      await configure('voxcpm2', 'geralt');
      const interrupted = await prepareText(projectId, longText);
      await click('Озвучить');
      await checkpoint(projectId, interrupted.run_id);
      await title(projectId, `native-e2e-speech-interrupt-ready:${runId}`);
      return;
    }
    if (project.title === `native-e2e-speech-interrupt-ready:${runId}`) {
      const interrupted = (await invoke('list_speech_runs_cmd', { projectId })).find(
        (r) => r.job?.status === 'failed' && (r.job.progress.processedItems ?? 0) > 0,
      );
      if (!interrupted)
        throw new Error('Interrupted worker was not recovered to a resumable failure');
      await select('Сохранённые запуски', interrupted.run_id);
      await click('Продолжить');
      await ready(projectId, interrupted.run_id);
      await playback();
      await title(projectId, `native-e2e-speech-complete:${runId}`);
      return;
    }
    const retained = (await invoke('list_speech_runs_cmd', { projectId })).filter(
      (r) => r.result?.ready,
    );
    if (retained.length !== 0 && retained.length !== 8)
      throw new Error('Native Speech matrix is incomplete; start a fresh sandbox');
    if (retained.length === 0) {
      const imported = await invoke('import_translation_source_cmd', {
        projectId,
        path: __NATIVE_E2E_SUBTITLE_PATH__,
        format: 'srt',
      });
      await waitForAsync(async () =>
        (await invoke('list_project_artifacts_cmd', { projectId, kind: 'originalSubtitle' })).some(
          (a) => a.id === imported.id && a.state === 'ready',
        ),
      );
      for (const engine of ['voxcpm2', 'espeech']) {
        for (const character of ['ciri', 'geralt']) {
          await configure(engine, character);
          const text = await prepareText(
            projectId,
            'Добрый вечер. Мы проверяем ясность русской речи.',
          );
          await click('Озвучить');
          await ready(projectId, text.run_id);
          await playback();
          await click('Субтитры');
          await click('Обновить источники');
          await select('Источник субтитров', `artifact:${imported.id}`);
          await click('Текст проверен');
          await click('Подготовить озвучку');
          const subtitles = await newest(projectId);
          if (subtitles.utterance_count !== 3 || subtitles.chunk_count <= 3)
            throw new Error('Subtitle plan did not preserve cues and split the long cue');
          await click('Озвучить');
          await ready(projectId, subtitles.run_id);
          await playback();
        }
      }
    }
    await configure('voxcpm2', 'ciri');
    const cancelled = await prepareText(projectId, longText);
    await click('Озвучить');
    await checkpoint(projectId, cancelled.run_id);
    await click('Отменить');
    await waitForAsync(
      async () => (await state(projectId, cancelled.run_id)).job?.status === 'cancelled',
    );
    const snapshot = await state(projectId, cancelled.run_id);
    if (snapshot.result) throw new Error('Cancelled run published partial audio');
    await title(projectId, `native-e2e-speech-cancelled:${runId}`);
  } catch (error) {
    const reason = (error instanceof Error ? error.message : String(error))
      .replace(/[\r\n]/g, ' ')
      .slice(0, 80);
    await invoke('native_e2e_checkpoint_cmd', { checkpoint: `scenario-failed:${reason}` }).catch(
      () => undefined,
    );
    await title(projectId, `native-e2e-speech-failed:${runId}`).catch(() => undefined);
    throw error;
  }
}

async function configure(engine: string, character: string) {
  await select('Движок и профиль', `native-plan-${engine}`);
  await select('Голос', `${character}_${engine}`);
  await select('Референс', `${engine}-neutral-01`);
}
async function prepareText(projectId: string, text: string) {
  await click('Текст');
  const textarea = await waitFor(() =>
    document.querySelector<HTMLTextAreaElement>('textarea[aria-label="Русский текст для озвучки"]'),
  );
  Object.getOwnPropertyDescriptor(HTMLTextAreaElement.prototype, 'value')?.set?.call(
    textarea,
    text,
  );
  textarea.dispatchEvent(new Event('input', { bubbles: true }));
  await click('Текст проверен');
  await click('Подготовить озвучку');
  return newest(projectId);
}
async function newest(projectId: string) {
  const runs = await invoke('list_speech_runs_cmd', { projectId });
  const run = runs.find((r) => !r.job);
  if (!run)
    throw new Error(`Speech plan was not saved: ${document.body.textContent?.slice(-1200)}`);
  return run;
}
async function checkpoint(projectId: string, runId: string) {
  await waitForAsync(async () => {
    const run = await state(projectId, runId);
    if (run.job?.status === 'failed') throw new Error(run.job.error ?? 'Speech worker failed');
    return (run.job?.progress.processedItems ?? 0) >= 1 && run.job?.status === 'running';
  });
}
async function ready(projectId: string, runId: string) {
  await waitForAsync(async () => {
    const run = await state(projectId, runId);
    if (run.job && ['failed', 'cancelled'].includes(run.job.status))
      throw new Error(run.job.error ?? 'Speech worker failed');
    return run.result?.ready && run.job?.status === 'completed';
  });
}
function state(projectId: string, runId: string) {
  return invoke('get_speech_run_cmd', { projectId, runId });
}
async function playback() {
  await click('Прослушать WAV');
  const audio = await waitFor(() => {
    const audio = document.querySelector('audio');
    return audio && audio.readyState >= HTMLMediaElement.HAVE_METADATA ? audio : null;
  });
  if (!(audio.duration > 0)) throw new Error('Ready WAV has no playable duration');
  await audio.play();
  await waitFor(() => audio.currentTime > 0.1);
  audio.pause();
}
async function title(projectId: string, title: string) {
  const current = await invoke('get_project_cmd', { projectId });
  await renameProject(projectId, title, current.revision);
}
async function select(label: string, value: string) {
  const element = await waitFor(() => {
    const el = document.querySelector<HTMLSelectElement>(`select[aria-label="${label}"]`);
    return el && [...el.options].some((o) => o.value === value) ? el : null;
  });
  const details = element.closest('details');
  if (details) details.open = true;
  Object.getOwnPropertyDescriptor(HTMLSelectElement.prototype, 'value')?.set?.call(element, value);
  element.dispatchEvent(new Event('change', { bubbles: true }));
  await delay(50);
}
async function click(label: string) {
  const element = await waitFor(() =>
    [...document.querySelectorAll('button')].find(
      (b) => b.textContent?.trim() === label && !b.disabled,
    ),
  );
  element.click();
  await delay(50);
  await waitFor(() => !element.isConnected || !element.disabled);
  const error = document.querySelector('[role="alert"]')?.textContent;
  if (error) throw new Error(error);
}
async function waitFor<T>(observe: () => T | undefined | null | false): Promise<T> {
  const deadline = Date.now() + 60_000;
  while (Date.now() < deadline) {
    if (document.body.textContent?.includes('TTS недоступен:'))
      throw new Error(document.body.textContent);
    const value = observe();
    if (value) return value;
    await delay(POLL_MS);
  }
  throw new Error(
    `Timed out waiting for the native Speech UI: ${document.body.textContent?.slice(-1200)}`,
  );
}
async function waitForAsync(observe: () => Promise<boolean | null | undefined>) {
  const deadline = Date.now() + TIMEOUT_MS;
  while (Date.now() < deadline) {
    if (await observe()) return;
    await delay(POLL_MS);
  }
  throw new Error('Timed out waiting for a saved Speech checkpoint/result');
}
function delay(ms: number) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}
