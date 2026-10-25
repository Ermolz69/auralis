import assert from 'node:assert/strict';
import { playback } from './media.mjs';
import { readyOrError } from './observe.mjs';

export async function exportVideo(page, assembly, remaining, quiescent) {
  const select = assembly.getByLabel('Видеоэкспорт', { exact: true });
  const ids = await select
    .locator('option')
    .evaluateAll((options) =>
      options.filter((option) => option.value).map((option) => option.value),
    );
  assert.equal(ids.length, 1, 'Select exactly one managed original video');
  await select.selectOption(ids[0]);
  await assembly.getByRole('button', { name: 'Измерить и показать тайминг', exact: true }).click();
  await readyOrError(
    assembly.getByText('Видео проверено:', { exact: false }),
    assembly,
    Math.min(180000, remaining()),
  );
  await assembly.getByRole('checkbox').check();
  await assembly.getByRole('button', { name: 'Подготовить сборку', exact: true }).click();
  await assembly.getByRole('button', { name: 'Собрать WAV и MP4', exact: true }).click();
  const play = assembly.getByRole('button', { name: 'Воспроизвести русское видео', exact: true });
  await readyOrError(play, assembly, remaining());
  await quiescent();
  await play.click();
  const observed = await playback(page, 'Видео с русской озвучкой');
  assert.ok(observed.video_width > 0 && observed.video_height > 0, 'Video frames did not decode');
  await select.selectOption('');
  return observed;
}
