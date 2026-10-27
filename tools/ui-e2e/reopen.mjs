import assert from 'node:assert/strict';
import { playback } from './media.mjs';
import { jobState } from './observe.mjs';
import { openSubtitleTool } from './navigation.mjs';

export function workflowRecords(report, spec) {
  const evidence = (id) => {
    const step = report.data.steps.find((item) => item.id === id);
    assert.equal(step?.status, 'passed', `Missing completed ${id}`);
    return step.evidence;
  };
  const records = spec.engines.map((selection) => {
    const engine = selection.engine_id;
    const saved = evidence(`${engine}-selection`);
    const wav = evidence(`${engine}-assembly`);
    assert.equal(saved.run_id, wav.run_id, 'Assembly selected a different speech run');
    return {
      ...selection,
      run_id: saved.run_id,
      wav_assembly_id: wav.assembly_id,
      ...(spec.export_video ? { video_assembly_id: evidence(`${engine}-video`).assembly_id } : {}),
    };
  });
  assert.equal(
    new Set(records.map((r) => r.run_id)).size,
    spec.engines.length,
    'Engines reused a speech run',
  );
  const ids = records.flatMap((r) => [
    r.wav_assembly_id,
    ...(spec.export_video ? [r.video_assembly_id] : []),
  ]);
  assert.ok(ids.every(Boolean));
  assert.equal(new Set(ids).size, ids.length, 'Outputs reused an assembly');
  return records;
}

export async function reopenPlayback(page, records, remaining) {
  await openSubtitleTool(page, 'Синтез речи');
  const observed = [];
  for (const record of records) {
    page.setDefaultTimeout(Math.min(30000, remaining()));
    const speech = page.getByRole('region', { name: 'Голоса синтеза речи', exact: true });
    await speech.getByLabel('Сохранённые запуски', { exact: true }).selectOption(record.run_id);
    await jobState(speech, 'Состояние озвучки', ['готово'], remaining());
    await speech.getByRole('button', { name: 'Прослушать WAV', exact: true }).click();
    const raw = await playback(page, 'Готовая озвучка');
    await page.getByLabel('Готовый исходный синтез', { exact: true }).selectOption(record.run_id);
    const assembly = page.getByRole('region', { name: 'Сборка итогового аудио', exact: true });
    await assembly
      .getByLabel('Сохранённые сборки', { exact: true })
      .selectOption(record.wav_assembly_id);
    await jobState(assembly, 'Состояние сборки', ['готово'], remaining());
    await assembly.getByRole('button', { name: 'Прослушать итоговый WAV', exact: true }).click();
    const wav = await playback(page, 'Итоговое аудио');
    let video;
    if (record.video_assembly_id) {
      await assembly
        .getByLabel('Сохранённые сборки', { exact: true })
        .selectOption(record.video_assembly_id);
      await jobState(assembly, 'Состояние сборки', ['готово'], remaining());
      await assembly
        .getByRole('button', { name: 'Воспроизвести русское видео', exact: true })
        .click();
      video = await playback(page, 'Видео с русской озвучкой');
      assert.ok(
        video.video_width > 0 && video.video_height > 0,
        'No decoded video frames after restart',
      );
    }
    observed.push({ ...record, raw, wav, ...(video ? { video } : {}) });
  }
  return { engines: observed };
}
