import assert from 'node:assert/strict';
import crypto from 'node:crypto';
import fsp from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import test from 'node:test';
import { verifyHistoryResults } from './translation-history-results.mjs';

test('strict WebVTT history retains protected CRLF bytes across edits and rejects changed timing', async () => {
  const tempBase = path.resolve(os.tmpdir());
  const root = await fsp.mkdtemp(path.join(tempBase, 'auralis-native-vtt-history-test-'));
  const dataRoot = path.join(root, 'app-data');
  const managedRoot = path.join(dataRoot, 'projects', 'translation');
  const model =
    'WEBVTT\r\n\r\nNOTE provenance\r\nnative fixture\r\n\r\n' +
    '00:00:01.000 --> 00:00:02.000\r\nЗдравствуйте.\r\n\r\n' +
    '00:00:03.000 --> 00:00:04.000\r\nДо свидания.\r\n';
  const outputs = [
    model,
    model.replace('До свидания.', 'Поздняя правка второй реплики.'),
    model.replace('Здравствуйте.', 'Правка первой реплики из исходной версии.'),
  ];
  const publications = outputs.map((bytes, index) => ({
    result_id: `result-${index + 1}`,
    result_revision: index + 1,
    run_id: 'one-run',
    artifact_id: `artifact-${index + 1}`,
    location_value: `translation/result-${index + 1}.vtt`,
    size_bytes: Buffer.byteLength(bytes),
  }));
  const results = new Map(
    outputs.map((bytes, index) => [
      publications[index].result_id,
      {
        result_id: publications[index].result_id,
        revision: index + 1,
        run_id: 'one-run',
        review_state: 'needs_review',
        output_sha256: crypto.createHash('sha256').update(bytes).digest('hex'),
      },
    ]),
  );
  const translate = {
    prepare: () => ({ get: (resultId) => results.get(resultId) }),
  };
  try {
    await fsp.mkdir(managedRoot, { recursive: true });
    for (const [index, bytes] of outputs.entries())
      await fsp.writeFile(path.join(managedRoot, `result-${index + 1}.vtt`), bytes);
    const observed = await verifyHistoryResults(dataRoot, translate, publications, 'vtt');
    assert.deepEqual(observed.modelLines, ['Здравствуйте.', 'До свидания.']);
    assert.equal(observed.outputs.length, 3);
    const changed = outputs[1].replace('00:00:03.000', '00:00:03.001');
    await fsp.writeFile(path.join(managedRoot, 'result-2.vtt'), changed);
    results.get('result-2').output_sha256 = crypto
      .createHash('sha256')
      .update(changed)
      .digest('hex');
    publications[1].size_bytes = Buffer.byteLength(changed);
    await assert.rejects(
      verifyHistoryResults(dataRoot, translate, publications, 'vtt'),
      /protected vtt bytes/,
    );
  } finally {
    if (!path.resolve(root).startsWith(`${tempBase}${path.sep}`))
      throw new Error('Temporary verification root escaped the system temp directory');
    await fsp.rm(root, { recursive: true, force: true });
  }
});
